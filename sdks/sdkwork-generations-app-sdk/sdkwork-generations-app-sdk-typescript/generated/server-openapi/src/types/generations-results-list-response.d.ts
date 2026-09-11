import type { GenerationResult } from './generation-result';
export interface GenerationsResultsListResponse {
    code: 0;
    data: unknown & {
        items: GenerationResult[];
        pageInfo: {
            mode: 'cursor';
            nextCursor?: string | null;
            hasMore: boolean;
        };
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=generations-results-list-response.d.ts.map