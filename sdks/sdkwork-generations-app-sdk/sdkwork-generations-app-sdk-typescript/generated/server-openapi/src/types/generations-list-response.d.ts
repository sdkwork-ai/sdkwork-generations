import type { GenerationRecord } from './generation-record';
export interface GenerationsListResponse {
    code: 0;
    data: unknown & {
        items: GenerationRecord[];
        pageInfo: {
            mode: 'cursor';
            nextCursor?: string | null;
            hasMore: boolean;
        };
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=generations-list-response.d.ts.map