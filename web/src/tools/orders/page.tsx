import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import {
  orderWords,
  type OrderOption,
  type OrderOptions,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { MarginNote } from "@/components/margin-note";
import type { Basis } from "@/overview/words";
import { useOrders } from "@/searches";
import { useSession } from "@/session";
import { ToolAbout } from "@/tools/about";
import { SearchActions, type Chosen } from "@/tools/act";
import { Options, type OptionRow } from "@/tools/options";
import type { ToolSearch } from "@/tools/search";
import { offeredName } from "@/workspace";
import { basisOf } from "@/year/search";
import { BasisSwitch } from "@/year/year";

const WORDS = orderWords();

/** The plan as it stands, then every order. */
function rowsOf(found: OrderOptions, basis: Basis): OptionRow<OrderOption>[] {
  return [
    {
      key: found.current,
      cells: [found.current, ...found.baseline[basis]],
      narrow: found.current,
      option: null,
    },
    ...found.options.map((option) => ({
      key: option.key,
      cells: [option.said, ...option.figures[basis]],
      narrow: option.said,
      option,
    })),
  ];
}

/** The highlighted order, as it is taken or written. */
function chosenOrder(path: string | null, option: OrderOption): Chosen {
  return {
    noun: "this order",
    question: option.question,
    described:
      "This order is written as a scenario over this plan, to a file of this name in your workspace.",
    offered: offeredName(path, `order-${option.key}`),
    take: (document) => document.takeOrder(option.order),
    scenario: (document, out) => document.orderScenario(out, option.order),
  };
}

/**
 * The Withdrawal Order tool: every order the plan's kinds of account can be
 * withdrawn from ranked best first, the highlighted order taken or written
 * as a scenario.
 */
export function OrdersPage({ title }: { title: string }) {
  const search: ToolSearch = useSearch({ from: "/tools/$page" });
  const navigate = useNavigate({ from: "/tools/$page" });
  const basis = basisOf(search);
  const { reading, issues, path } = useSession();
  const isValid = issues.length === 0;
  const plan = useMemo(() => reading.document?.planText() ?? "", [reading]);
  const found = useOrders(plan, isValid);
  const reply = found.data;
  const highlighted =
    reply?.options.find((each) => each.key === search.order) ??
    reply?.options[0];
  const isCurrent = !found.isFetching && !found.isPlaceholderData;
  const rows = useMemo(() => reply && rowsOf(reply, basis), [reply, basis]);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
          <ToolAbout about={WORDS.about} />
        </div>
        <BasisSwitch />
      </div>
      <section aria-labelledby="orders" className="min-w-0 space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex items-baseline gap-3">
            <h2 id="orders" className="text-lg font-semibold">
              Orders
            </h2>
            {found.isFetching && (
              <span className="text-muted-foreground text-sm">Searching…</span>
            )}
          </div>
          {highlighted && isValid && !found.error && (
            <SearchActions
              key={highlighted.key}
              chosen={chosenOrder(path, highlighted)}
              isCurrent={isCurrent}
            />
          )}
        </div>
        {!isValid ? (
          <p className="text-muted-foreground">
            The orders are searched once the plan&apos;s issues are fixed; the{" "}
            <Link to="/overview" className="underline underline-offset-4">
              Overview
            </Link>{" "}
            lists them.
          </p>
        ) : found.error && !found.isFetching ? (
          <MarginNote zone="caution" role="alert">
            <p className="text-sm">{found.error.message}</p>
          </MarginNote>
        ) : !reply ? (
          <p className="text-muted-foreground">{WORDS.nothing_searched}</p>
        ) : (
          <Options
            label="Orders"
            columns={reply.columns}
            rows={rows ?? []}
            narrowFigure={reply.columns.indexOf(WORDS.against_plan)}
            highlighted={highlighted}
            highlight={(option) => {
              void navigate({
                search: (kept) => ({ ...kept, order: option.key }),
                replace: true,
              });
            }}
          />
        )}
      </section>
    </div>
  );
}
