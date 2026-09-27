import { useRouteContext } from "@tanstack/react-router";

/** A page still to be built, saying what it will show. */
export function Placeholder({
  title,
  holds,
}: {
  title: string;
  holds: string | null;
}) {
  return (
    <section className="max-w-prose space-y-2">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      {holds && <p className="text-muted-foreground">{holds}</p>}
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

/** The page of a group that its route's `$page` named. */
export function GroupedPage() {
  const { page } = useRouteContext({ strict: false });
  if (!page) return null;
  return <Placeholder title={page.title} holds={page.holds} />;
}
