import type { GenerationCommandResponse } from './generation-command-response';

export interface GenerationsVideosAvatarResponse202 {
  code: 0;
  data: unknown & { item: GenerationCommandResponse; };
  /** Server-owned request correlation id. */
  traceId: string;
}
