import { useNavigate, useRouter } from "@tanstack/react-router";
import { useCallback } from "react";

import { isGroup, type Page, type Tab } from "@/nav";
import { keptSearch } from "@/year/search";

/**
 * Where a tab's link goes - its page, or the group's page named, its first
 * where none - carrying what of the address its route keeps.
 */
export function useTabTarget() {
  const router = useRouter();
  return useCallback(
    (tab: Tab, page?: Page) => {
      const keeps = router.routesByPath[tab.path].options.staticData?.keeps;
      const slug = isGroup(tab) ? (page ?? tab.pages[0])?.slug : undefined;
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
