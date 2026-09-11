export interface CreateGenerationCommandRequest {
    tenantId: string;
    organizationId?: string;
    prompt: string;
    model?: string;
    inputAssetIds?: string[];
    parameters?: Record<string, unknown>;
}
//# sourceMappingURL=create-generation-command-request.d.ts.map