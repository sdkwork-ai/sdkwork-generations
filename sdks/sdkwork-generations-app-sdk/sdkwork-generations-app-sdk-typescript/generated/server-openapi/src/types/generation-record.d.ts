import type { GenerationModality } from './generation-modality';
import type { GenerationStatus } from './generation-status';
export interface GenerationRecord {
    id: string;
    tenantId: string;
    organizationId?: string;
    userId: string;
    modality: GenerationModality;
    operationType: string;
    sourceProvider?: string;
    sourceJobId?: string;
    promptPreview?: string;
    status: GenerationStatus;
    favorite?: boolean;
    resultCount?: number;
    createdAt: string;
    updatedAt: string;
}
//# sourceMappingURL=generation-record.d.ts.map