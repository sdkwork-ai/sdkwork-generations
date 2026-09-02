//! Web framework bootstrap for generations route surfaces.

use crate::generated::{APP_ROUTES, BACKEND_ROUTES, COMBINED_ROUTES};
use std::sync::Arc;

use axum::Router;
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_web_axum::{with_web_request_context, WebFrameworkLayer};
use sdkwork_web_core::{
    DomainContextInjector, HttpRouteManifest, SecurityPolicy, WebEnvironment, WebRequestContext,
    WebRequestContextProfile,
};

use sdkwork_intelligence_generations_service::GenerationsRequestContext;

fn canonical_lifecycle_environment(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "dev" | "development" | "local" => "development".to_owned(),
        "test" | "testing" => "test".to_owned(),
        "stage" | "staging" => "staging".to_owned(),
        "prod" | "production" | "live" => "production".to_owned(),
        other => other.to_owned(),
    }
}

fn parse_web_environment(value: Option<String>) -> WebEnvironment {
    match canonical_lifecycle_environment(value.as_deref().unwrap_or("")).as_str() {
        "development" => WebEnvironment::Dev,
        "test" => WebEnvironment::Test,
        // Demo is an isolated showcase tier, not production-like: it gets the
        // relaxed showcase posture instead of production assembly validation.
        "demo" => WebEnvironment::Test,
        // Staging/prod keep the strict fail-closed production posture.
        "staging" | "production" => WebEnvironment::Prod,
        _ => WebEnvironment::Prod,
    }
}

fn first_nonempty_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn resolve_web_environment_from_process_env() -> WebEnvironment {
    parse_web_environment(first_nonempty_env(&[
        "SDKWORK_ENVIRONMENT",
        "SDKWORK_GENERATIONS_ENVIRONMENT",
        "SDKWORK_ENV",
        "ENVIRONMENT",
    ]))
}

fn configured_cors_origins_from_process_env() -> Vec<String> {
    sdkwork_web_bootstrap::cors_allowed_origins_from_env(&["SDKWORK_CORS_ALLOWED_ORIGINS"])
}

fn generations_service_security_policy(environment: &WebEnvironment) -> SecurityPolicy {
    let configured_origins = configured_cors_origins_from_process_env();
    let has_configured_origins = !configured_origins.is_empty();
    let requires_exact_origins = matches!(environment, WebEnvironment::Prod)
        || matches!(environment, WebEnvironment::Test) && has_configured_origins;
    let cors_environment = if matches!(environment, WebEnvironment::Test) && has_configured_origins
    {
        WebEnvironment::Prod
    } else {
        environment.clone()
    };
    let cors = sdkwork_web_bootstrap::security_policy_for_environment(
        &cors_environment,
        configured_origins,
    )
    .cors;
    let use_development_security_policy = matches!(environment, WebEnvironment::Dev)
        || matches!(environment, WebEnvironment::Test) && !has_configured_origins;
    let mut security_policy = if use_development_security_policy {
        SecurityPolicy::default()
    } else {
        SecurityPolicy::production()
    };
    security_policy.cors = cors;
    if use_development_security_policy {
        security_policy
            .cross_site
            .reject_untrusted_state_changing_origins = false;
        security_policy.cross_site.reject_cookie_auth_without_origin = false;
    }
    security_policy
}

pub fn app_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(APP_ROUTES)
}

pub fn backend_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(BACKEND_ROUTES)
}

pub fn combined_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(COMBINED_ROUTES)
}

fn generations_web_request_context_profile(
    environment: WebEnvironment,
) -> WebRequestContextProfile {
    WebRequestContextProfile {
        environment,
        ..WebRequestContextProfile::default()
    }
}

#[derive(Clone, Default)]
struct GenerationsRequestContextInjector;

impl DomainContextInjector for GenerationsRequestContextInjector {
    fn inject(&self, request: &mut axum::extract::Request, context: &WebRequestContext) {
        if let Some(gen_context) = generations_request_context_from_web_request(context) {
            request.extensions_mut().insert(gen_context);
        }
    }
}

pub fn generations_request_context_injector() -> Arc<dyn DomainContextInjector> {
    Arc::new(GenerationsRequestContextInjector)
}

fn generations_request_context_from_web_request(
    context: &WebRequestContext,
) -> Option<GenerationsRequestContext> {
    use sdkwork_intelligence_generations_service::GenerationsHttpRequestContext;
    let principal = context.principal.as_ref()?;
    Some(GenerationsRequestContext {
        http: GenerationsHttpRequestContext {
            tenant_id: principal.tenant_id().to_string(),
            user_id: principal.user_id().to_string(),
            trace_id: context
                .trace_id
                .as_deref()
                .unwrap_or_default()
                .to_string(),
        },
    })
}

pub fn wrap_router_with_web_framework(
    resolver: IamWebRequestContextResolver,
    route_manifest: HttpRouteManifest,
    router: Router,
) -> Router {
    let environment = resolve_web_environment_from_process_env();
    let security_policy = generations_service_security_policy(&environment);
    let layer = WebFrameworkLayer::new(resolver)
        .with_profile(generations_web_request_context_profile(environment))
        .with_security_policy(security_policy)
        .with_route_manifest(route_manifest)
        .with_domain_injector(generations_request_context_injector());
    with_web_request_context(router, layer)
}

pub async fn wrap_router_with_web_framework_from_env(
    route_manifest: HttpRouteManifest,
    router: Router,
) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_web_request_context_resolver_from_env().await;
    wrap_router_with_web_framework(resolver, route_manifest, router)
}

pub async fn build_served_combined_router(router: Router) -> Router {
    wrap_router_with_web_framework_from_env(combined_route_manifest(), router).await
}
