import type { Action, Names } from "@bindings";

/** The dollars figures are shown in: today's, or those of their own year. */
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

/** The year whose actions to show: this one, held within the plan's years. */
export function actionsYear(first: number, last: number, now: number): number {
  return Math.min(Math.max(now, first), last);
}

/** One of a year's actions, said as what to do. */
export interface Step {
  verb: string;
  amount: number;
  detail: string;
}

export function step(action: Action, names: Names): Step {
  const account = (id: string) => names.accounts[id] ?? id;
  switch (action.kind) {
    case "transfer":
      return {
        verb: "Move",
        amount: action.amount,
        detail: `${account(action.from)} → ${account(action.to)}`,
      };
    case "conversion":
      return {
        verb: "Convert",
        amount: action.amount,
        detail: `${account(action.from)} → ${account(action.to)}`,
      };
    case "rmd":
      return {
        verb: "Take the required distribution",
        amount: action.amount,
        detail: `from ${account(action.account)}`,
      };
    case "withdrawal":
      return {
        verb: "Withdraw",
        amount: action.amount,
        detail: `from ${account(action.account)}`,
      };
    case "contribution":
      return {
        verb: "Contribute",
        amount: action.employee + action.employer,
        detail:
          action.employer > 0
            ? `to ${account(action.account)}, ${dollars(action.employer)} of it from the employer`
            : `to ${account(action.account)}`,
      };
    case "surplus":
      return {
        verb: "Save what is left",
        amount: action.amount,
        detail: `into ${account(action.account)}`,
      };
  }
}
