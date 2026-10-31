export interface GenerationPricingEstimateLine {
  /** Billing meter code the line was priced against. */
  meterCode: string;
  /** Quantity the charge scales with, as a decimal string. */
  quantity: string;
  /** `requested` when the client declared the quantity; `single_unit` when the request carries none, so one unit is shown as a lower bound. */
  quantityBasis: 'requested' | 'single_unit';
  /** Unit the quantity is denominated in (second, character, item). */
  quantityUnit: string;
  /** Customer-charge unit price for the meter, decimal string. */
  unitPrice: string;
  /** Pricing currency of `unitPrice` (ISO 4217). */
  currencyCode: string;
  /** Divisor applied to `unitPrice` per unit (e.g. 1000000 for per-million meters). */
  unitSize: string;
  /** Cash amount for the line (`unitPrice × quantity / unitSize`). */
  amount: string;
  /** Token-bank points for the line, decimal string in points. */
  points: string;
  /** Exact micro-points (1 point = 1e6 micro) for the line. */
  pointsMicros: number;
  /** Pricing tier the rate was conditioned on, when the catalog declared one. */
  tierCode?: string;
}
