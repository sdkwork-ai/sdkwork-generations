/** Command body for a generation creation. Tenant and organization context resolve from the authenticated session (API_SPEC §10.0/§14); clients MUST NOT supply context selector fields. */
export interface CreateGenerationCommandRequest {
  prompt: string;
  model?: string;
  inputAssetIds?: string[];
  parameters?: Record<string, unknown>;
}
