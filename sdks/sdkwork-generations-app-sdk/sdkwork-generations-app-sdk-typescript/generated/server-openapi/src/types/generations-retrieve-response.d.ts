import type { GenerationRecord } from './generation-record';
export interface GenerationsRetrieveResponse {
    code: 0;
    data: unknown & {
        item: GenerationRecord;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=generations-retrieve-response.d.ts.map