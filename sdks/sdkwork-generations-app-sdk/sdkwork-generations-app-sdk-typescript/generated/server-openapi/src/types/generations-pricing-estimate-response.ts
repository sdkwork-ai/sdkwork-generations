import type { GenerationPricingEstimate } from './generation-pricing-estimate';

export interface GenerationsPricingEstimateResponse {
  code: 0;
  data: unknown & { item: GenerationPricingEstimate; };
  /** Server-owned request correlation id. */
  traceId: string;
}
