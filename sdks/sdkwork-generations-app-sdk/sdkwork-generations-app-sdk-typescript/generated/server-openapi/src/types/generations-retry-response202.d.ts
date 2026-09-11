import type { GenerationCommandResponse } from './generation-command-response';
export interface GenerationsRetryResponse202 {
    code: 0;
    data: unknown & {
        item: GenerationCommandResponse;
    };
    /** Server-owned request correlation id. */
    traceId: string;
}
//# sourceMappingURL=generations-retry-response202.d.ts.map