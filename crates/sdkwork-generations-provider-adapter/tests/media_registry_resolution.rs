//! Proves the media app-api path (image/video/music/sfx all ride the
//! CloudRouter media gateway) resolves its base URL through the database
//! registry and follows an online failover without a process restart.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sdkwork_generations_provider_adapter::{CloudRouterMediaGateway, GatewaySettings};
use sdkwork_utils_rust::service_base_url::{
    install_http_service_registry, HttpServiceRegistry, RegistryHttpTarget,
};

struct SwitchableRegistry {
    second: AtomicBool,
}

impl HttpServiceRegistry for SwitchableRegistry {
    fn resolve_http(&self, service: &str) -> Option<RegistryHttpTarget> {
        if service != "cloudrouter" {
            return None;
        }
        let second = self.second.load(Ordering::SeqCst);
        Some(RegistryHttpTarget {
            service: "cloudrouter".to_owned(),
            instance_id: if second {
                "registry-media-2"
            } else {
                "registry-media-1"
            }
            .to_owned(),
            base_url: if second {
                "https://registry-b.example.com"
            } else {
                "https://registry-a.example.com"
            }
            .to_owned(),
        })
    }

    fn report_failure(&self, _target: &RegistryHttpTarget, _reason: &str) {}

    fn report_success(&self, _target: &RegistryHttpTarget) {}
}

#[test]
fn media_gateway_follows_the_registry_target_and_online_failover() {
    let registry = Arc::new(SwitchableRegistry {
        second: AtomicBool::new(false),
    });
    assert!(
        install_http_service_registry(registry.clone()),
        "the registry slot is process-global and this test binary owns it"
    );

    let gateway = CloudRouterMediaGateway::new(&GatewaySettings {
        base_url: "https://fallback.example.com".to_owned(),
        api_key: None,
        auth_token: None,
        access_token: None,
    })
    .expect("media gateway builds");

    assert_eq!(gateway.base_url(), "https://registry-a.example.com");

    registry.second.store(true, Ordering::SeqCst);
    assert_eq!(
        gateway.base_url(),
        "https://registry-b.example.com",
        "the gateway rebuilds its SDK client when the registry target changes"
    );
}
