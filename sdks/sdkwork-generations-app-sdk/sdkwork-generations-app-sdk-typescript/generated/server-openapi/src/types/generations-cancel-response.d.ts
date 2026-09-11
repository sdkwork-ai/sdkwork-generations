import type { GenerationRecord } from './generation-record';
export interface GenerationsCancelResponse {
    code: 0;
    data: unknown & {
        item: GenerationRecord;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=generations-cancel-response.d.ts.map