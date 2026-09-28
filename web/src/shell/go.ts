import { useNavigate, useRouter } from "@tanstack/react-router";
import { useCallback } from "react";

import { isGroup, type Page, type Tab } from "@/nav";
import { keptSearch } from "@/year/search";

/** Where a tab's link goes: its page, or the group's page named, its first where none. */
export function tabTarget(tab: Tab, page?: Page) {
  const slug = isGroup(tab) ? (page ?? tab.pages[0])?.slug : undefined;
  return {
    to: tab.path,
    params: slug === undefined ? {} : { page: slug },
  };
}

/** Goes to a tab's page, or one of its group's, carrying what its route keeps. */
export function useGoTo() {
  const router = useRouter();
  const navigate = useNavigate();
  return useCallback(
    (tab: Tab, page?: Page) => {
      const keeps = router.routesByPath[tab.path].options.staticData?.keeps;
      void navigate({
        ...tabTarget(tab, page),
        search: (prev) => keptSearch(prev, keeps ?? []),
      });
    },
    [router, navigate],
  );
}
