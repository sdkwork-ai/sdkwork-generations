/** Command body for a generation creation. Tenant and organization context resolve from the authenticated session (API_SPEC §10.0/§14); clients MUST NOT supply context selector fields. */
export interface CreateGenerationCommandRequest {
  prompt: string;
  model?: string;
  inputAssetIds?: string[];
  parameters?: Record<string, unknown>;
  /** Client-scoped conversation grouping key. Every command of one multi-turn conversation carries the same value, so the record list can reassemble the conversation after a reload. Opaque to the backend; never a context selector. */
  conversationId?: string;
}
