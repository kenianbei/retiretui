/** The dollars a figure is shown in: today's, or those of its own year. */
export type Basis = "today" | "nominal";

export const BASIS_LABEL: Record<Basis, string> = {
  today: "today's $",
  nominal: "nominal $",
};

const DOLLARS = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
});

const SHARE = new Intl.NumberFormat("en-US", {
  style: "percent",
  maximumFractionDigits: 0,
});

export function dollars(amount: number): string {
  return DOLLARS.format(amount);
}

export function share(rate: number): string {
  return SHARE.format(rate);
}
