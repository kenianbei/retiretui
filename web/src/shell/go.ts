import { useLocation, useNavigate, useRouter } from "@tanstack/react-router";
import { useCallback, useEffect } from "react";

import { isGroup, type Page, TABS, type Tab } from "@/nav";
import { noteShown, shownIn } from "@/shell/shown";
import { keptSearch } from "@/year/search";

/**
 * Keeps the page of a group the address is on as the one the group's tab
 * comes back to from anywhere else.
 */
export function useShownKept() {
  const pathname = useLocation({ select: (location) => location.pathname });
  useEffect(() => {
    for (const tab of TABS.filter(isGroup)) noteShown(tab, pathname);
  }, [pathname]);
}

/**
 * Where a tab's link goes - its page, or the group's page named, the one
 * the group is on or was last on where none - carrying what of the address
 * its route keeps.
 */
export function useTabTarget() {
  const router = useRouter();
  return useCallback(
    (tab: Tab, page?: Page) => {
      const keeps = router.routesByPath[tab.path].options.staticData?.keeps;
      const { pathname } = router.state.location;
      const slug = isGroup(tab)
        ? (page ?? shownIn(tab, pathname))?.slug
        : undefined;
      return {
        to: tab.path,
        params: slug === undefined ? {} : { page: slug },
        search: (prev: Record<string, unknown>) =>
          keptSearch(prev, keeps ?? []),
      };
    },
    [router],
  );
}

/** Goes to a tab's page, or one of its group's, as its link does. */
export function useGoTo() {
  const target = useTabTarget();
  const navigate = useNavigate();
  return useCallback(
    (tab: Tab, page?: Page) => {
      void navigate(target(tab, page));
    },
    [target, navigate],
  );
}
