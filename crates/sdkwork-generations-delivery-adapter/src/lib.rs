//! Cloud Router app-api adapter for the generation delivery policy.
//!
//! `APP_SDK_INTEGRATION_SPEC.md` §5.2.1: the delivery policy is resolved while
//! the creating caller's verified dual tokens exist. This adapter is therefore
//! constructed **per caller** (`for_caller`): it sends the caller's
//! `Authorization` bearer and `Access-Token` to the storage-center resolve
//! endpoint the app-api exposes
//! (`GET /app/v3/api/storage/generation_delivery`), and fails closed with a
//! `DeliveryNotConfigured` diagnostic when no caller is bound — a background
//! flow must use [`DefaultRemoteOnlyDeliveryResolver`] instead.
//!
//! The endpoint is the caller-scoped one (not an admin or internal surface) so
//! the answered policy is always the caller's own tenant/organization/user
//! scope; a deployment-level key is never used here.

use std::time::Duration;

use reqwest::Client;
use sdkwork_intelligence_generations_service::domain::delivery::{
    GenerationDeliveryMode, GenerationDeliverySnapshot,
};
use sdkwork_intelligence_generations_service::error::GenerationsError;
use sdkwork_intelligence_generations_service::ports::delivery::{
    GenerationDeliveryFuture, GenerationDeliveryResolveRequest, GenerationDeliveryResolver,
};
use sdkwork_utils_rust::caller_identity::CallerCredentials;
use sdkwork_utils_rust::service_base_url::registry_base_url_or;

/// Explicit app-api origin override; when unset the `cloudrouter` registry entry
/// is used (the app-api is served from the same origin as the open-api).
pub const APP_API_BASE_URL_ENV: &str = "SDKWORK_GENERATIONS_CLOUDROUTER_APP_API_BASE_URL";
/// Resolve path on the app-api surface.
pub const RESOLVE_PATH: &str = "/app/v3/api/storage/generation_delivery";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct CloudRouterDeliveryPolicyResolver {
    client: Client,
    /// Registry-first base URL, with the configured value as degradation.
    configured_base_url: String,
    caller: CallerCredentials,
}

impl CloudRouterDeliveryPolicyResolver {
    /// Builds the adapter from the environment.
    ///
    /// # Errors
    ///
    /// Fails when neither `SDKWORK_GENERATIONS_CLOUDROUTER_APP_API_BASE_URL` nor a
    /// healthy `cloudrouter` registry entry yields a base URL.
    pub fn from_env() -> Result<Self, GenerationsError> {
        let configured = std::env::var(APP_API_BASE_URL_ENV).unwrap_or_default();
        let base_url = registry_base_url_or("cloudrouter", configured.trim());
        if base_url.trim().is_empty() {
            return Err(GenerationsError::DeliveryNotConfigured);
        }
        Self::with_base_url(base_url, DEFAULT_TIMEOUT)
    }

    /// Builds the adapter against an explicit origin (tests, pinned hosts).
    ///
    /// # Errors
    ///
    /// Returns [`GenerationsError::Delivery`] when the HTTP client cannot be
    /// constructed.
    pub fn with_base_url(
        base_url: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, GenerationsError> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| {
                GenerationsError::Delivery(format!(
                    "delivery policy HTTP client init failed: {error}"
                ))
            })?;
        Ok(Self {
            client,
            configured_base_url: base_url.into().trim().trim_end_matches('/').to_owned(),
            caller: CallerCredentials::default(),
        })
    }

    /// Binds the verified caller of one creation request.
    #[must_use]
    pub fn for_caller(&self, caller: &CallerCredentials) -> Self {
        let mut resolver = self.clone();
        resolver.caller = caller.clone();
        resolver
    }

    fn endpoint(&self) -> String {
        let base_url = registry_base_url_or("cloudrouter", &self.configured_base_url);
        format!("{}{RESOLVE_PATH}", base_url.trim_end_matches('/'))
    }

    async fn fetch(
        &self,
        request: GenerationDeliveryResolveRequest,
    ) -> Result<GenerationDeliverySnapshot, GenerationsError> {
        if self.caller.is_empty() {
            return Err(GenerationsError::DeliveryNotConfigured);
        }
        let mut query = vec![("mediaKind", request.media_kind.clone())];
        if let Some(vendor_code) = request
            .vendor_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            query.push(("vendorCode", vendor_code.to_owned()));
        }
        let mut http_request = self.client.get(self.endpoint()).query(&query);
        // The verified dual-token pair is the only credential on this hop
        // (APP_SDK_INTEGRATION_SPEC section 5.2.1); wire_headers renders
        // Authorization: Bearer plus Access-Token, and never an API key.
        for (name, value) in self.caller.wire_headers() {
            http_request = http_request.header(name, value);
        }
        let response = http_request.send().await.map_err(|error| {
            GenerationsError::Delivery(format!("delivery policy resolve request failed: {error}"))
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(GenerationsError::Delivery(format!(
                "delivery policy resolve answered http status {}",
                status.as_u16()
            )));
        }
        let envelope: ResolveEnvelope = response.json().await.map_err(|error| {
            GenerationsError::Delivery(format!(
                "delivery policy resolve response is invalid: {error}"
            ))
        })?;
        if envelope.code != 0 {
            return Err(GenerationsError::Delivery(format!(
                "delivery policy resolve answered code {}",
                envelope.code
            )));
        }
        let data = envelope.data.ok_or_else(|| {
            GenerationsError::Delivery(
                "delivery policy resolve response carries no payload".to_owned(),
            )
        })?;
        let delivery_mode =
            GenerationDeliveryMode::parse(&data.delivery_mode).ok_or_else(|| {
                GenerationsError::Delivery(format!(
                    "delivery policy mode `{}` is not supported",
                    data.delivery_mode
                ))
            })?;
        let mut snapshot = GenerationDeliverySnapshot::remote_only_default(
            request.generation_id,
            request.tenant_id,
            request.organization_id,
            request.user_id,
            request.media_kind,
            request.vendor_code,
        );
        snapshot.delivery_mode = delivery_mode;
        snapshot.drive_space_id = normalize(data.drive_space_id);
        snapshot.drive_folder_node_id = normalize(data.drive_folder_node_id);
        snapshot.download_deadline_seconds = data
            .download_deadline_seconds
            .filter(|seconds| *seconds > 0);
        snapshot.vendor_asset_ttl_seconds =
            data.vendor_asset_ttl_seconds.filter(|seconds| *seconds > 0);
        if let Some(source) = data.source_scope {
            snapshot.source_scope_type = normalize(Some(source.scope_type));
            snapshot.source_scope_id = normalize(Some(source.scope_id));
        }
        Ok(snapshot)
    }
}

fn normalize(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(serde::Deserialize)]
struct ResolveEnvelope {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    data: Option<ResolveData>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveData {
    #[serde(default)]
    delivery_mode: String,
    #[serde(default)]
    drive_space_id: Option<String>,
    #[serde(default)]
    drive_folder_node_id: Option<String>,
    #[serde(default)]
    download_deadline_seconds: Option<i32>,
    #[serde(default)]
    vendor_asset_ttl_seconds: Option<i32>,
    #[serde(default)]
    source_scope: Option<SourceScope>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceScope {
    #[serde(default)]
    scope_type: String,
    #[serde(default)]
    scope_id: String,
}

impl GenerationDeliveryResolver for CloudRouterDeliveryPolicyResolver {
    fn resolve<'a>(
        &'a self,
        request: GenerationDeliveryResolveRequest,
    ) -> GenerationDeliveryFuture<'a, GenerationDeliverySnapshot> {
        Box::pin(async move { self.fetch(request).await })
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    fn caller() -> CallerCredentials {
        CallerCredentials::new(
            Some("caller-auth-token".to_owned()),
            Some("caller-access-token".to_owned()),
        )
    }

    fn request() -> GenerationDeliveryResolveRequest {
        GenerationDeliveryResolveRequest {
            generation_id: "gen-77".to_owned(),
            tenant_id: "100001".to_owned(),
            organization_id: "7".to_owned(),
            user_id: "42".to_owned(),
            media_kind: "video".to_owned(),
            vendor_code: Some("vidu".to_owned()),
        }
    }

    async fn serve_once(
        status_line: &'static str,
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("listener");
        let addr: SocketAddr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 2048];
            loop {
                let read = socket.read(&mut chunk).await.expect("read");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&buffer).to_string();
            let response = format!(
                "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.expect("write");
            socket.flush().await.expect("flush");
            request
        });
        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn a_resolver_without_a_caller_fails_closed() {
        let resolver = CloudRouterDeliveryPolicyResolver::with_base_url(
            "http://127.0.0.1:1",
            Duration::from_millis(200),
        )
        .expect("resolver");
        let error = resolver
            .resolve(request())
            .await
            .err()
            .expect("a background flow must not use the caller-scoped resolver");
        assert!(
            matches!(error, GenerationsError::DeliveryNotConfigured),
            "{error}"
        );
    }

    #[tokio::test]
    async fn maps_the_resolved_policy_and_forwards_the_caller_pair() {
        let body = r#"{"code":0,"data":{"deliveryMode":"to_drive","driveSpaceId":"space-1","driveFolderNodeId":"folder-9","downloadDeadlineSeconds":600,"vendorAssetTtlSeconds":3600,"sourceScope":{"scopeType":"organization","scopeId":"7"}},"traceId":"trace-1"}"#;
        let (base_url, server) = serve_once("HTTP/1.1 200 OK", body).await;
        let resolver =
            CloudRouterDeliveryPolicyResolver::with_base_url(base_url, Duration::from_secs(5))
                .expect("resolver")
                .for_caller(&caller());
        let snapshot = resolver
            .resolve(request())
            .await
            .expect("resolved snapshot");
        assert_eq!(snapshot.delivery_mode, GenerationDeliveryMode::ToDrive);
        assert_eq!(snapshot.drive_space_id.as_deref(), Some("space-1"));
        assert_eq!(snapshot.drive_folder_node_id.as_deref(), Some("folder-9"));
        assert_eq!(snapshot.download_deadline_seconds, Some(600));
        assert_eq!(snapshot.vendor_asset_ttl_seconds, Some(3_600));
        assert_eq!(snapshot.source_scope_type.as_deref(), Some("organization"));
        assert_eq!(snapshot.source_scope_id.as_deref(), Some("7"));
        assert_eq!(snapshot.effective_download_deadline_seconds(), Some(600));

        let captured = server.await.expect("server");
        let lower = captured.to_ascii_lowercase();
        assert!(lower.contains("get /app/v3/api/storage/generation_delivery?"));
        assert!(lower.contains("mediakind=video"));
        assert!(lower.contains("vendorcode=vidu"));
        assert!(lower.contains("authorization: bearer caller-auth-token"));
        assert!(lower.contains("access-token: caller-access-token"));
        assert!(!lower.contains("x-api-key"));
    }

    #[tokio::test]
    async fn a_rejected_resolve_is_reported_not_retried_forever() {
        let (base_url, server) = serve_once("HTTP/1.1 503 Service Unavailable", "{}").await;
        let resolver =
            CloudRouterDeliveryPolicyResolver::with_base_url(base_url, Duration::from_secs(5))
                .expect("resolver")
                .for_caller(&caller());
        let error = resolver
            .resolve(request())
            .await
            .err()
            .expect("a rejected resolve is an error");
        assert!(matches!(error, GenerationsError::Delivery(_)), "{error}");
        server.await.expect("server");
    }

    #[tokio::test]
    async fn a_payload_less_response_is_an_error() {
        let (base_url, server) = serve_once("HTTP/1.1 200 OK", r#"{"code":0,"data":null}"#).await;
        let resolver =
            CloudRouterDeliveryPolicyResolver::with_base_url(base_url, Duration::from_secs(5))
                .expect("resolver")
                .for_caller(&caller());
        let error = resolver
            .resolve(request())
            .await
            .err()
            .expect("missing payload is an error");
        assert!(matches!(error, GenerationsError::Delivery(_)), "{error}");
        server.await.expect("server");
    }
}
