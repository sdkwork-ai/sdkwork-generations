import type { GenerationPricingEstimateLine } from './generation-pricing-estimate-line';

export interface GenerationPricingEstimate {
  /** True when every chargeable meter resolved a rate; false answers carry `unpricedReason`. */
  priced: boolean;
  /** Total points for the command, decimal string. */
  pointsTotal: string;
  /** Exact total in micro-points (1 point = 1e6 micro). */
  pointsMicros: number;
  /** Pricing currency the lines were charged in, when priced. */
  currencyCode?: string;
  /** Model the estimate was priced against (echo of the request). */
  model?: string;
  /** Pricing catalog key the model resolved to (`vendor/model`). */
  catalogKey?: string;
  /** Vendor segment of the resolved catalog key. */
  vendorCode?: string;
  /** Billing region the rate was resolved in. */
  regionCode?: string;
  /** Per-meter breakdown, ordered by the catalog's primary-meter preference. */
  lines: GenerationPricingEstimateLine[];
  /** Diagnosis when `priced` is false. */
  unpricedReason?: string;
  /** When the estimate was computed (RFC 3339). */
  quotedAt: string;
}
