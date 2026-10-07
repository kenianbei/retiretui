/** A tab holding a group of pages, as far as remembering one of them goes. */
interface Paged<P> {
  path: string;
  pages: readonly P[];
}

/** The page each group was last on, for as long as the app is loaded. */
const lastShown = new Map<string, string>();

/** The page of `group` that `pathname` is the address of. */
function pageAt<P extends { slug: string }>(
  group: Paged<P>,
  pathname: string,
): P | undefined {
  return group.pages.find(
    (each) => group.path.replace("$page", each.slug) === pathname,
  );
}

/** Keeps the page of `group` at `pathname` as the one its tab comes back to. */
export function noteShown(group: Paged<{ slug: string }>, pathname: string) {
  const page = pageAt(group, pathname);
  if (page) lastShown.set(group.path, page.slug);
}

/**
 * The page a group's tab shows from `pathname`: the group's page there, the
 * one last shown in it from anywhere else, and its first until one has been.
 */
export function shownIn<P extends { slug: string }>(
  group: Paged<P>,
  pathname: string,
): P | undefined {
  const slug = lastShown.get(group.path);
  return (
    pageAt(group, pathname) ??
    group.pages.find((each) => each.slug === slug) ??
    group.pages[0]
  );
}
