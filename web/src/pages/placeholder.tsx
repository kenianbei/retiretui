import { useParams } from "@tanstack/react-router";

import { pageOf, type Page } from "@/nav";

/** A page still to be built, saying what it will show. */
export function Placeholder({
  title,
  holds,
}: {
  title: string;
  holds: string;
}) {
  return (
    <section className="max-w-prose space-y-2">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <p className="text-muted-foreground">{holds}</p>
      <p className="text-muted-foreground text-sm">
        This page is not built yet.
      </p>
    </section>
  );
}

export function NotFound() {
  return (
    <Placeholder
      title="Page not found"
      holds="There is no page at this address."
    />
  );
}

/** The page of a group that its route's `$page` names. */
export function GroupedPage({ pages }: { pages: readonly Page[] }) {
  const { page: slug = "" } = useParams({ strict: false });
  const page = pageOf(pages, slug);
  return <Placeholder title={page.title} holds={page.holds} />;
}
