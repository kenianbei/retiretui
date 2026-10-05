import { Link, useRouteContext, useSearch } from "@tanstack/react-router";
import type { Sort } from "@wasm/retiretui_wasm.js";
import { ChevronLeft, Plus } from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ItemForm } from "@/plan/form";
import { ReadOut } from "@/plan/read-out";
import { ItemTable } from "@/plan/table";
import { useSession } from "@/session";

/** A list domain: its table, and the highlighted item read out beside it. */
function ListDomain({ slug, purpose }: { slug: string; purpose: string }) {
  const { item } = useSearch({ from: "/plan/$page" });
  const [sort, setSort] = useState<Sort | null>(null);
  const { reading } = useSession();
  const table = useMemo(
    () => reading.document?.table(slug, sort) ?? null,
    [reading, slug, sort],
  );
  if (!table) return null;
  const highlighted =
    table.rows.find((row) => row.index === item) ?? table.rows[0];

  const add = (
    <Button size="sm" variant="outline" asChild>
      <Link to="/plan/$page" params={{ page: slug }} search={{ edit: "new" }}>
        <Plus aria-hidden />
        Add
      </Link>
    </Button>
  );
  if (table.rows.length === 0) {
    return (
      <div className="space-y-3">
        <p className="text-muted-foreground max-w-prose">{purpose}</p>
        {add}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className={item !== undefined ? "max-md:hidden" : undefined}>
        {add}
      </div>
      <div className="grid grid-cols-1 gap-6 @3xl/page:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <div
          className={cn("@container", item !== undefined && "max-md:hidden")}
        >
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
              known={highlighted.known}
            />
          </div>
        )}
      </div>
    </div>
  );
}

/** The page of one of the plan's editing domains. */
export function DomainPage() {
  const { page } = useRouteContext({ from: "/plan/$page" });
  const { edit, field } = useSearch({ from: "/plan/$page" });
  return (
    <section className="space-y-6">
      {edit !== undefined && (
        <ItemForm
          key={`${page.slug}:${String(edit)}`}
          slug={page.slug}
          edit={edit}
          field={field}
        />
      )}
      <h1 className="text-2xl font-semibold tracking-tight">{page.title}</h1>
      {page.holds === null ? (
        <ReadOut slug={page.slug} index={0} />
      ) : (
        <ListDomain slug={page.slug} purpose={page.holds} />
      )}
    </section>
  );
}
