import type { MediaResource } from './media-resource';
export interface GenerationResult {
    id: string;
    generationId: string;
    resultType: string;
    driveSpaceId?: string;
    driveNodeId?: string;
    driveUri?: string;
    resourceSnapshot?: MediaResource;
    assetId?: string;
    previewText?: string;
    createdAt: string;
}
//# sourceMappingURL=generation-result.d.ts.map