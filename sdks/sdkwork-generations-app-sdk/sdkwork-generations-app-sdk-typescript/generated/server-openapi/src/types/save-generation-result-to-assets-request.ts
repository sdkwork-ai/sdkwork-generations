/** Save-to-assets command. Tenant context resolves from the authenticated session; clients MUST NOT supply context selector fields. */
export interface SaveGenerationResultToAssetsRequest {
  collectionId?: string;
  title?: string;
  tags?: string[];
}
