import type { GenerationRecord } from './generation-record';
export interface GenerationsFavoriteResponse {
    code: 0;
    data: unknown & {
        item: GenerationRecord;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=generations-favorite-response.d.ts.map