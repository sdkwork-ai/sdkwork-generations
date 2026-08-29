//! Transport wiring for the generations MCP service (stdio + streamable HTTP).

use std::sync::Arc;

use rmcp::{
    service::{RunningService, ServerInitializeError},
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    RoleServer, ServiceExt,
};

use crate::handler::GenerationsMcpService;

/// Streamable HTTP MCP service type for gateway mounting.
pub type GenerationsMcpHttpService =
    StreamableHttpService<GenerationsMcpService, LocalSessionManager>;

/// Wrap the generations MCP service as a streamable HTTP service.
pub fn streamable_http_service(
    service: GenerationsMcpService,
    config: StreamableHttpServerConfig,
) -> GenerationsMcpHttpService {
    StreamableHttpService::new(
        move || Ok(service.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    )
}

/// Serve the generations MCP service over stdio.
pub async fn serve_stdio(
    service: GenerationsMcpService,
) -> Result<RunningService<RoleServer, GenerationsMcpService>, ServerInitializeError> {
    service.serve(rmcp::transport::stdio()).await
}
