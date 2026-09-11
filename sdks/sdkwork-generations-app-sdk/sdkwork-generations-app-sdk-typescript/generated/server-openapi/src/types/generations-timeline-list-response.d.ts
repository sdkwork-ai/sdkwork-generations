import type { GenerationTimelineEvent } from './generation-timeline-event';
export interface GenerationsTimelineListResponse {
    code: 0;
    data: unknown & {
        items: GenerationTimelineEvent[];
        pageInfo: {
            mode: 'cursor';
            nextCursor?: string | null;
            hasMore: boolean;
        };
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=generations-timeline-list-response.d.ts.map