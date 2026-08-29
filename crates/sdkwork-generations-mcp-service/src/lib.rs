//! Model Context Protocol adapter for sdkwork-generations.
//!
//! Exposes the four agent-facing generation capabilities — image generation,
//! video generation, speech synthesis, and music generation — as MCP tools
//! backed by the generations service (vendor dispatch happens through the
//! provider adapters and the cloudrouter Rust SDK).

use std::sync::Arc;

use sdkwork_intelligence_generations_service::domain::models::{
    CreateGenerationCommandRequest, GenerationModality, GenerationRecord, GenerationResult,
};
use sdkwork_intelligence_generations_service::error::GenerationsError;

pub mod dto;
pub mod handler;
pub mod kernel_invoke;
pub mod kernel_mcp;
pub mod port;
pub mod transport;

pub use dto::*;
pub use handler::GenerationsMcpService;
pub use kernel_mcp::{
    generations_kernel_mcp_provider, GenerationsMcpProvider, GENERATIONS_MCP_PROVIDER_ID,
    GENERATIONS_MCP_SERVER_ID,
};
pub use port::{
    GenerationsMcpPort, InMemoryGenerationsMcpPort, StateGenerationsMcpPort,
};
pub use transport::{serve_stdio, streamable_http_service};

use sdkwork_intelligence_generations_service::GenerationsServiceState;

/// Shared alias so handlers stay decoupled from concrete service wiring.
pub type SharedGenerationsMcpPort = Arc<dyn GenerationsMcpPort>;

/// Build an MCP service bound to a generations service state.
pub fn generations_mcp_service(state: GenerationsServiceState) -> GenerationsMcpService {
    GenerationsMcpService::new(Arc::new(StateGenerationsMcpPort::new(state)))
}

/// Build an MCP service bound to a caller-supplied port.
pub fn generations_mcp_service_from_port(port: std::sync::Arc<dyn GenerationsMcpPort>) -> GenerationsMcpService {
    GenerationsMcpService::new(port)
}

/// Create a generation command from MCP tool inputs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn create_command(
    tenant_id: &str,
    prompt: &str,
    model: Option<&str>,
    parameters: serde_json::Value,
    input_asset_ids: Option<Vec<String>>,
) -> CreateGenerationCommandRequest {
    CreateGenerationCommandRequest {
        tenant_id: tenant_id.to_string(),
        organization_id: None,
        prompt: prompt.to_string(),
        model: model.map(str::to_string),
        input_asset_ids,
        parameters: Some(parameters),
    }
}

/// Serialize a generation record and its results for MCP tool output.
pub(crate) fn generation_payload(
    record: &GenerationRecord,
    results: &[GenerationResult],
) -> crate::dto::GenerationsToolOutput {
    let record_value = serde_json::to_value(record).unwrap_or(serde_json::Value::Null);
    let result_values = results
        .iter()
        .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
        .collect::<Vec<_>>();
    let media_urls = results
        .iter()
        .filter_map(|result| {
            result
                .resource_snapshot
                .as_ref()
                .and_then(|resource| {
                    resource.url.clone().or_else(|| resource.public_url.clone())
                })
        })
        .collect();
    crate::dto::GenerationsToolOutput {
        generation: record_value,
        results: result_values,
        media_urls,
    }
}

/// Map a generations error into the MCP tool error payload.
pub(crate) fn tool_error(error: &GenerationsError) -> crate::dto::GenerationsMcpToolError {
    crate::dto::GenerationsMcpToolError {
        code: error.platform_code(),
        message: error.to_string(),
    }
}

/// Convenience helper resolving a modality slug into the domain enum.
pub(crate) fn modality_or_image(value: &str) -> GenerationModality {
    GenerationModality::parse(value).unwrap_or(GenerationModality::Image)
}

#[cfg(test)]
mod tests {
    use crate::port::InMemoryGenerationsMcpPort;
    use crate::{generations_mcp_service_from_port, GenerationsMcpService};

    #[tokio::test]
    async fn exposes_seven_generation_tools() {
        let service = GenerationsMcpService::new(std::sync::Arc::new(
            InMemoryGenerationsMcpPort::new(),
        ));
        let names = service
            .tools()
            .iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(names.len(), 7);
        for expected in [
            "generation.image.create",
            "generation.image.retrieve",
            "generation.video.create",
            "generation.video.retrieve",
            "generation.speech.create",
            "generation.music.create",
            "generation.music.retrieve",
        ] {
            assert!(names.contains(&expected.to_string()), "missing {expected}");
        }
    }
}

#[cfg(test)]
mod kernel_mcp_tests {
    use std::sync::Arc;

    use sdkwork_agent_kernel::{McpProvider, ToolCall};

    use crate::port::InMemoryGenerationsMcpPort;
    use crate::{GenerationsMcpProvider, GENERATIONS_MCP_PROVIDER_ID, GENERATIONS_MCP_SERVER_ID};

    #[test]
    fn provider_follows_rig_mcp_convention() {
        let provider = GenerationsMcpProvider::new(Arc::new(InMemoryGenerationsMcpPort::new()));

        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, GENERATIONS_MCP_PROVIDER_ID);
        assert!(manifest.capabilities.contains(&"mcp.tools".to_string()));

        let servers = provider.list_servers().expect("servers list");
        assert_eq!(servers.len(), 1);
        let server = &servers[0];
        assert_eq!(server.server_id, GENERATIONS_MCP_SERVER_ID);
        assert!(server.permits_tool("mcp__generations__image.create"));
        assert!(!server.permits_tool("mcp__generations__unknown.tool"));
        assert!(!server.permits_tool("mcp__other__image.create"));
    }

    #[test]
    fn tool_descriptors_use_mcp_namespace_and_json_schema() {
        let provider = GenerationsMcpProvider::new(Arc::new(InMemoryGenerationsMcpPort::new()));
        let tools = provider
            .list_tools(GENERATIONS_MCP_SERVER_ID)
            .expect("tools list");

        assert_eq!(tools.len(), 7);
        for tool in &tools {
            assert!(
                tool.tool_id.starts_with("mcp__generations__"),
                "tool id {0} must follow mcp__<server>__<tool> naming",
                tool.tool_id
            );
            assert_eq!(tool.provider_id, GENERATIONS_MCP_PROVIDER_ID);
            let schema = tool
                .input_schema
                .as_ref()
                .expect("input schema present");
            assert_eq!(
                schema.dialect.as_deref(),
                Some("https://json-schema.org/draft/2020-12/schema")
            );
            assert!(schema.document_json().is_some(), "schema document parses");
        }
        let image = tools
            .iter()
            .find(|tool| tool.tool_id == "mcp__generations__image.create")
            .expect("image.create tool");
        assert!(image
            .policy_categories
            .contains(&"media.generation.image".to_string()));
    }

    #[test]
    fn invoke_routes_to_generations_port() {
        let port: Arc<dyn crate::port::GenerationsMcpPort> =
            Arc::new(InMemoryGenerationsMcpPort::new());
        let provider = GenerationsMcpProvider::new(Arc::clone(&port));

        let result = provider
            .invoke_tool(
                GENERATIONS_MCP_SERVER_ID,
                ToolCall::new(
                    "call-1",
                    "mcp__generations__image.create",
                    r#"{"prompt": "a cat", "vendor": "openai"}"#,
                ),
            )
            .expect("invoke succeeds");
        assert_eq!(result.status, "succeeded");
        let output: serde_json::Value =
            serde_json::from_str(&result.output).expect("output parses");
        assert!(output["generation"]["id"].is_string());
        assert_eq!(output["generation"]["modality"], "image");
    }

    #[test]
    fn invoke_unknown_tool_fails_closed() {
        let provider = GenerationsMcpProvider::new(Arc::new(InMemoryGenerationsMcpPort::new()));
        let result = provider
            .invoke_tool(
                GENERATIONS_MCP_SERVER_ID,
                ToolCall::new("call-2", "mcp__generations__unknown.tool", "{}"),
            )
            .expect("invoke returns a failed result");
        assert_eq!(result.status, "failed");
        assert!(result.error.is_some());
    }
}
