//! Kernel `McpProvider` implementation for sdkwork-generations.
//!
//! Follows the sdkwork-kernel RIG agent MCP convention (`sdkwork-agent-kernel`
//! `McpProvider` trait): the generations capabilities are exposed as one MCP
//! server whose tools use the `mcp__<server>__<tool>` naming contract, with
//! typed JSON Schema tool descriptors and a fail-closed default surface that
//! only exposes the tools this provider actually implements.

use std::sync::Arc;

use rmcp::schemars;
use sdkwork_agent_kernel::{
    KernelError, KernelResult, McpProvider, McpResourceContent, McpResourceDescriptor,
    McpServerDescriptor, McpTransportKind, ProviderHealth, ProviderManifest, ToolCall,
    ToolDescriptor, ToolResult, ToolSchema,
};

use crate::dto::{
    GenerateImageInput, GenerateMusicInput, GenerateVideoInput, GenerationRetrieveInput,
    SynthesizeSpeechInput,
};
use crate::port::GenerationsMcpPort;

/// Provider id advertised in the kernel provider manifest.
pub const GENERATIONS_MCP_PROVIDER_ID: &str = "generations.mcp";
/// MCP server id exposed through `list_servers`.
pub const GENERATIONS_MCP_SERVER_ID: &str = "generations";
/// Capability resource URI describing supported vendors and surfaces.
pub const GENERATIONS_MCP_CAPABILITY_URI: &str = "generations://capabilities";

/// Default tool timeout for media generations (vendor tasks may run minutes).
pub const GENERATIONS_MCP_TOOL_TIMEOUT_MS: u64 = 180_000;

const MCP_PROVIDER_FAMILY: &str = "mcp";
const MCP_PROVIDER_NAME: &str = "sdkwork-generations-mcp";
const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Kernel MCP provider routing generations tools to the generations service.
pub struct GenerationsMcpProvider {
    port: Arc<dyn GenerationsMcpPort>,
}

impl GenerationsMcpProvider {
    /// Build the provider over a generations port.
    pub fn new(port: Arc<dyn GenerationsMcpPort>) -> Self {
        Self { port }
    }

    /// Provider manifest following the RIG provider convention.
    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            GENERATIONS_MCP_PROVIDER_ID,
            MCP_PROVIDER_FAMILY,
            MCP_PROVIDER_NAME,
            env!("CARGO_PKG_VERSION"),
            vec![
                "mcp.tools".to_string(),
                "mcp.resources".to_string(),
            ],
        )
    }

    /// Tool descriptors exposed for the generations server.
    pub fn tool_descriptors(&self) -> Vec<ToolDescriptor> {
        vec![
            self.descriptor(
                "image.create",
                "Generate images (text-to-image or image edit).",
                serde_json::to_value(schemars::schema_for!(GenerateImageInput)).expect("image schema serializes"),
                "media.generation.image",
            ),
            self.descriptor(
                "image.retrieve",
                "Retrieve an image generation by generation id.",
                serde_json::to_value(schemars::schema_for!(GenerationRetrieveInput)).expect("retrieve schema serializes"),
                "media.generation.image.read",
            ),
            self.descriptor(
                "video.create",
                "Generate videos (text-to-video, image-to-video, or start-end frames).",
                serde_json::to_value(schemars::schema_for!(GenerateVideoInput)).expect("video schema serializes"),
                "media.generation.video",
            ),
            self.descriptor(
                "video.retrieve",
                "Retrieve a video generation by generation id.",
                serde_json::to_value(schemars::schema_for!(GenerationRetrieveInput)).expect("retrieve schema serializes"),
                "media.generation.video.read",
            ),
            self.descriptor(
                "speech.create",
                "Synthesize speech audio from text.",
                serde_json::to_value(schemars::schema_for!(SynthesizeSpeechInput)).expect("speech schema serializes"),
                "media.generation.voice",
            ),
            self.descriptor(
                "music.create",
                "Generate music tracks (text-to-music or lyrics-to-music).",
                serde_json::to_value(schemars::schema_for!(GenerateMusicInput)).expect("music schema serializes"),
                "media.generation.music",
            ),
            self.descriptor(
                "music.retrieve",
                "Retrieve a music generation by generation id.",
                serde_json::to_value(schemars::schema_for!(GenerationRetrieveInput)).expect("retrieve schema serializes"),
                "media.generation.music.read",
            ),
        ]
    }

    fn descriptor(
        &self,
        tool_name: &str,
        description: &str,
        schema: serde_json::Value,
        policy_category: &str,
    ) -> ToolDescriptor {
        let schema_id = format!("{GENERATIONS_MCP_SERVER_ID}.{tool_name}");
        ToolDescriptor::new(
            sdkwork_agent_kernel::mcp_tool_name(GENERATIONS_MCP_SERVER_ID, tool_name),
            GENERATIONS_MCP_PROVIDER_ID,
            format!("generations.{tool_name}"),
            sdkwork_agent_kernel::SideEffectLevel::SideEffectful,
        )
        .with_description(description)
        .with_input_schema(
            ToolSchema::json_schema(schema_id)
                .with_document(schema)
                .with_dialect(JSON_SCHEMA_DIALECT),
        )
        .with_policy_categories(vec![policy_category.to_string()])
        .with_timeout_ms(GENERATIONS_MCP_TOOL_TIMEOUT_MS)
    }

    fn ensure_server(server_id: &str) -> KernelResult<()> {
        if server_id == GENERATIONS_MCP_SERVER_ID {
            Ok(())
        } else {
            Err(KernelError::CapabilityMissing {
                capability_id: format!("mcp.server.{server_id}"),
            })
        }
    }

    fn resolve_tool(&self, tool_id: &str) -> Option<String> {
        let parsed = sdkwork_agent_kernel::parse_mcp_tool_name(tool_id)?;
        if parsed.server_id != GENERATIONS_MCP_SERVER_ID {
            return None;
        }
        Some(parsed.tool_name)
    }
}

impl McpProvider for GenerationsMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        GenerationsMcpProvider::provider_manifest(self)
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_servers(&self) -> KernelResult<Vec<McpServerDescriptor>> {
        let mut server = McpServerDescriptor::new(
            GENERATIONS_MCP_SERVER_ID,
            GENERATIONS_MCP_PROVIDER_ID,
            McpTransportKind::StreamableHttp,
        )
        .with_capability("mcp.tools")
        .with_capability("mcp.resources")
        .with_tool_timeout_ms(GENERATIONS_MCP_TOOL_TIMEOUT_MS);
        for tool in self.tool_descriptors() {
            server = server.with_enabled_tool(tool.tool_id);
        }
        Ok(vec![server])
    }

    fn list_tools(&self, server_id: &str) -> KernelResult<Vec<ToolDescriptor>> {
        Self::ensure_server(server_id)?;
        Ok(self.tool_descriptors())
    }

    fn invoke_tool(&self, server_id: &str, call: ToolCall) -> KernelResult<ToolResult> {
        Self::ensure_server(server_id)?;
        let Some(tool_name) = self.resolve_tool(&call.tool_id) else {
            return Err(KernelError::CapabilityMissing {
                capability_id: call.tool_id.clone(),
            });
        };

        let output = crate::kernel_invoke::invoke(&self.port, &tool_name, &call.arguments);
        match output {
            Ok(payload) => Ok(ToolResult::succeeded(call.tool_call_id, payload)),
            Err(error) => Ok(ToolResult::failed(call.tool_call_id, error)),
        }
    }

    fn list_resources(&self, server_id: &str) -> KernelResult<Vec<McpResourceDescriptor>> {
        Self::ensure_server(server_id)?;
        Ok(vec![McpResourceDescriptor::new(
            GENERATIONS_MCP_CAPABILITY_URI,
            "Generations capabilities",
            "text/plain",
        )
        .with_description(
            "Supported media generation vendors and dispatch surfaces (openai, \
             nano-banana, vidu, kling, volcengine, suno).",
        )])
    }

    fn read_resource(&self, server_id: &str, uri: &str) -> KernelResult<McpResourceContent> {
        Self::ensure_server(server_id)?;
        if uri != GENERATIONS_MCP_CAPABILITY_URI {
            return Err(KernelError::CapabilityMissing {
                capability_id: uri.to_string(),
            });
        }
        Ok(McpResourceContent::new(
            GENERATIONS_MCP_CAPABILITY_URI,
            "text/plain",
            crate::kernel_invoke::CAPABILITY_DOCUMENT,
        )
        .with_trust_level(sdkwork_agent_kernel::TrustLevel::TrustedHost))
    }
}

/// Convenience constructor mirroring the service-state MCP wiring.
pub fn generations_kernel_mcp_provider(port: Arc<dyn GenerationsMcpPort>) -> GenerationsMcpProvider {
    GenerationsMcpProvider::new(port)
}
