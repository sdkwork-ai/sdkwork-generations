//! Request context for the generations service.

use sdkwork_utils_rust::caller_identity::CallerCredentials;

/// HTTP request context injected by the web framework.
#[derive(Debug, Clone)]
pub struct GenerationsHttpRequestContext {
    /// Authenticated tenant id.
    pub tenant_id: String,
    /// Authenticated organization id, when the principal carries a team
    /// scope. Personal scopes leave this `None`; pricing resolution treats
    /// the absent organization as the tenant's personal (0) scope.
    pub organization_id: Option<String>,
    /// Authenticated user id.
    pub user_id: String,
    /// Request correlation id.
    pub trace_id: String,
    /// Verified caller credentials for the outbound provider call
    /// (`APP_SDK_INTEGRATION_SPEC.md` §5.2.1). Never serialized, never logged;
    /// empty on a background/worker path that uses a declared service identity.
    pub caller: CallerCredentials,
}

/// Business-level request context.
///
/// Carries the authenticated principal identity and trace correlation
/// through the service and port boundaries.
#[derive(Debug, Clone)]
pub struct GenerationsRequestContext {
    pub http: GenerationsHttpRequestContext,
}

impl GenerationsRequestContext {
    /// Create a new request context from the HTTP context.
    pub fn new(http: GenerationsHttpRequestContext) -> Self {
        Self { http }
    }

    /// Attach the verified caller credentials to this context.
    #[must_use]
    pub fn with_caller(mut self, caller: CallerCredentials) -> Self {
        self.http.caller = caller;
        self
    }

    /// The caller's dual credentials for outbound provider calls.
    #[must_use]
    pub fn caller(&self) -> &CallerCredentials {
        &self.http.caller
    }

    /// Construct from individual fields (personal scope, no organization).
    pub fn from_parts(tenant_id: String, user_id: String, trace_id: String) -> Self {
        Self::from_parts_with_organization(tenant_id, None, user_id, trace_id)
    }

    /// Construct with an explicit organization scope.
    pub fn from_parts_with_organization(
        tenant_id: String,
        organization_id: Option<String>,
        user_id: String,
        trace_id: String,
    ) -> Self {
        Self {
            http: GenerationsHttpRequestContext {
                tenant_id,
                organization_id,
                user_id,
                trace_id,
                caller: CallerCredentials::default(),
            },
        }
    }
}
