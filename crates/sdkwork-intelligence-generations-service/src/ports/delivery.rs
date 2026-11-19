//! Port for resolving the storage-center delivery policy of one generation.
//!
//! The implementation lives in the composition host (an HTTP adapter calling the
//! Cloud Router app-api resolve endpoint with the caller's verified dual
//! tokens). When it is not wired, [`DefaultRemoteOnlyDeliveryResolver`] keeps the
//! cost-saving default: generated media stays at the vendor, no Drive copy.

use std::future::Future;
use std::pin::Pin;

use crate::domain::delivery::GenerationDeliverySnapshot;
use crate::error::GenerationsError;

pub type GenerationDeliveryFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, GenerationsError>> + Send + 'a>>;

/// The generation whose delivery policy must be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationDeliveryResolveRequest {
    pub generation_id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub user_id: String,
    /// `all` / `image` / `video` / `audio`.
    pub media_kind: String,
    /// Provider vendor code, when the dispatch plan already knows it.
    pub vendor_code: Option<String>,
}

pub trait GenerationDeliveryResolver: Send + Sync {
    fn resolve<'a>(
        &'a self,
        request: GenerationDeliveryResolveRequest,
    ) -> GenerationDeliveryFuture<'a, GenerationDeliverySnapshot>;
}

/// The cost-saving resolver used when no storage center is wired.
pub struct DefaultRemoteOnlyDeliveryResolver;

impl GenerationDeliveryResolver for DefaultRemoteOnlyDeliveryResolver {
    fn resolve<'a>(
        &'a self,
        request: GenerationDeliveryResolveRequest,
    ) -> GenerationDeliveryFuture<'a, GenerationDeliverySnapshot> {
        Box::pin(async move {
            Ok(GenerationDeliverySnapshot::remote_only_default(
                request.generation_id,
                request.tenant_id,
                request.organization_id,
                request.user_id,
                request.media_kind,
                request.vendor_code,
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::delivery::GenerationDeliveryMode;

    #[tokio::test]
    async fn the_default_resolver_keeps_generated_media_remote() {
        let resolver = DefaultRemoteOnlyDeliveryResolver;
        let snapshot = resolver
            .resolve(GenerationDeliveryResolveRequest {
                generation_id: "gen-9".to_owned(),
                tenant_id: "100001".to_owned(),
                organization_id: "7".to_owned(),
                user_id: "42".to_owned(),
                media_kind: "video".to_owned(),
                vendor_code: Some("vidu".to_owned()),
            })
            .await
            .expect("default snapshot");
        assert_eq!(snapshot.delivery_mode, GenerationDeliveryMode::RemoteOnly);
        assert_eq!(snapshot.generation_id, "gen-9");
        assert_eq!(snapshot.tenant_id, "100001");
        assert_eq!(snapshot.organization_id, "7");
        assert_eq!(snapshot.user_id, "42");
        assert_eq!(snapshot.media_kind, "video");
        assert_eq!(snapshot.vendor_code.as_deref(), Some("vidu"));
        assert!(snapshot.source_scope_type.is_none());
    }
}
