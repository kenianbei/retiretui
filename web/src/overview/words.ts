/** The dollars a figure is shown in: today's, or those of its own year. */
export type Basis = "today" | "nominal";

const DOLLARS = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
});

export function dollars(amount: number): string {
  return DOLLARS.format(amount);
}
