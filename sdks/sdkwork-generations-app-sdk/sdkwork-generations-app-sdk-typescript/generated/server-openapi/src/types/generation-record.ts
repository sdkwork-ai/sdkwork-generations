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
  /** Client-scoped conversation grouping key. Every command of one multi-turn conversation carries the same value, so the record list can reassemble the conversation after a reload. Opaque to the backend; never a context selector. Present when the creating command carried one. */
  conversationId?: string;
}
