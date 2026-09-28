import { Link, useRouteContext, useSearch } from "@tanstack/react-router";
import { ChevronLeft, Plus } from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { ReadOut } from "@/plan/read-out";
import type { TableSort } from "@/plan/sort";
import { ItemTable } from "@/plan/table";
import { useSession } from "@/session";

/** A list domain: its table, and the highlighted item read out beside it. */
function ListDomain({ slug, purpose }: { slug: string; purpose: string }) {
  const { document, revision } = useSession();
  const { item } = useSearch({ from: "/plan/$page" });
  const [sort, setSort] = useState<TableSort | null>(null);
  const table = useMemo(
    () =>
      document?.table(slug, sort?.column, sort?.isDescending ?? false) ?? null,
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the document changes in place
    [document, revision, slug, sort],
  );
  if (!table) return null;
  const highlighted =
    table.rows.find((row) => row.index === item) ?? table.rows[0];

  return (
    <div className="space-y-4">
      <Button size="sm" variant="outline" asChild>
        <Link to="/plan/$page" params={{ page: slug }} search={{ edit: "new" }}>
          <Plus aria-hidden />
          Add
        </Link>
      </Button>
      {table.rows.length === 0 ? (
        <p className="text-muted-foreground max-w-prose">{purpose}</p>
      ) : (
        <div className="grid gap-6 md:grid-cols-[minmax(0,3fr)_minmax(18rem,2fr)]">
          <div className={item !== undefined ? "max-md:hidden" : undefined}>
            <ItemTable
              slug={slug}
              table={table}
              sort={sort}
              onSort={setSort}
              highlighted={highlighted?.index}
            />
          </div>
          {highlighted && (
            <div className={item === undefined ? "max-md:hidden" : undefined}>
              <Link
                to="/plan/$page"
                params={{ page: slug }}
                className="text-muted-foreground mb-3 inline-flex items-center gap-1 text-sm md:hidden"
              >
                <ChevronLeft aria-hidden className="size-4" />
                Every item
              </Link>
              <ReadOut
                slug={slug}
                index={highlighted.index}
                name={highlighted.name}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/** The page of one of the plan's editing domains. */
export function DomainPage() {
  const { page } = useRouteContext({ from: "/plan/$page" });
  return (
    <section className="max-w-6xl space-y-4">
      <h1 className="text-2xl font-semibold tracking-tight">{page.title}</h1>
      {page.holds === null ? (
        <div className="max-w-2xl">
          <ReadOut slug={page.slug} index={0} />
        </div>
      ) : (
        <ListDomain slug={page.slug} purpose={page.holds} />
      )}
    </section>
  );
}
