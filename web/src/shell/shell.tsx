import { Link, Outlet, useLocation } from "@tanstack/react-router";

import type { ReactNode } from "react";

import { DraftNotices, UnsavedQuestion } from "@/draft/notices";
import { DraftToolbar } from "@/draft/toolbar";
import { FileActionsProvider } from "@/files/actions";
import { FileMenu } from "@/files/menu";
import { Start } from "@/files/start";
import { cn } from "@/lib/utils";
import { useSession } from "@/session";
import { yearSearch } from "@/year/search";
import {
  TABS,
  isGroup,
  isWithin,
  isYearTab,
  type GroupTab,
  type Page,
  type Tab,
} from "@/nav";

function useIsActive() {
  const { pathname } = useLocation();
  return (tab: Tab) => isWithin(tab, pathname);
}

interface TabLinkProps {
  tab: Tab;
  /** The group's page to go to; its first where none is named. */
  page?: Page;
  className: string;
  children: ReactNode;
  isCurrent?: boolean;
}

/** A link to a tab's own page, or to one of its group's. */
function TabLink({ tab, page, className, children, isCurrent }: TabLinkProps) {
  const slug = isGroup(tab) ? (page ?? tab.pages[0])?.slug : undefined;
  return (
    <Link
      to={tab.path}
      params={slug === undefined ? {} : { page: slug }}
      search={(prev) => (isYearTab(tab) ? yearSearch(prev) : {})}
      className={className}
      aria-current={isCurrent ? "page" : undefined}
    >
      {children}
    </Link>
  );
}

function Sidebar() {
  const isActive = useIsActive();
  return (
    <nav
      aria-label="Main"
      className="bg-card hidden border-r px-3 py-4 md:block"
    >
      <p className="px-3 pb-4 font-semibold tracking-tight">RetireTui</p>
      <ul className="space-y-1">
        {TABS.map((tab) => (
          <li key={tab.path}>
            <TabLink
              tab={tab}
              isCurrent={isActive(tab) && !isGroup(tab)}
              className={cn(
                "hover:bg-accent flex items-center gap-3 rounded-md px-3 py-2 text-sm",
                isActive(tab) && "bg-accent font-semibold",
              )}
            >
              <tab.icon aria-hidden className="size-4" />
              {tab.title}
            </TabLink>
            {isGroup(tab) && isActive(tab) && (
              <PageLinks tab={tab} variant="sidebar" />
            )}
          </li>
        ))}
      </ul>
    </nav>
  );
}

function BottomBar() {
  const isActive = useIsActive();
  return (
    <nav
      aria-label="Main"
      className="bg-card fixed inset-x-0 bottom-0 z-10 border-t pb-[env(safe-area-inset-bottom)] md:hidden"
    >
      <ul className="grid grid-cols-5">
        {TABS.map((tab) => (
          <li key={tab.path}>
            <TabLink
              tab={tab}
              isCurrent={isActive(tab)}
              className={cn(
                "text-muted-foreground flex flex-col items-center gap-1 border-t-2 border-transparent py-2 text-xs",
                isActive(tab) && "border-primary text-foreground font-semibold",
              )}
            >
              <tab.icon aria-hidden className="size-5" />
              {tab.title}
            </TabLink>
          </li>
        ))}
      </ul>
    </nav>
  );
}

const PAGE_LINK = {
  sidebar: {
    list: "mt-1 ml-5 space-y-0.5 border-l pl-2",
    link: "hover:bg-accent block rounded-md px-3 py-1.5 text-sm",
    current: "bg-accent font-semibold",
  },
  chips: {
    list: "flex gap-2",
    link: "block rounded-full border px-3 py-1 text-sm whitespace-nowrap",
    current: "bg-primary text-primary-foreground border-primary",
  },
};

/** A grouped tab's pages: nested in the sidebar, or chips above the page on a phone. */
function PageLinks({
  tab,
  variant,
}: {
  tab: GroupTab;
  variant: keyof typeof PAGE_LINK;
}) {
  const { pathname } = useLocation();
  const style = PAGE_LINK[variant];
  return (
    <ul className={style.list}>
      {tab.pages.map((page) => {
        const isCurrent = pathname.endsWith(`/${page.slug}`);
        return (
          <li key={page.slug}>
            <TabLink
              tab={tab}
              page={page}
              isCurrent={isCurrent}
              className={cn(style.link, isCurrent && style.current)}
            >
              {page.title}
            </TabLink>
          </li>
        );
      })}
    </ul>
  );
}

/** The current group's pages above the page, where there is no sidebar. */
function GroupPages() {
  const isActive = useIsActive();
  const tab = TABS.filter(isGroup).find(isActive);
  if (!tab) return null;
  return (
    <nav
      aria-label={tab.title}
      className="-mx-4 mb-4 overflow-x-auto px-4 md:hidden"
    >
      <PageLinks tab={tab} variant="chips" />
    </nav>
  );
}

export function Shell() {
  const { document } = useSession();
  const isNewPlan = useLocation({
    select: (location) => location.pathname.startsWith("/new/"),
  });
  return (
    <FileActionsProvider>
      <div className="min-h-dvh md:grid md:grid-cols-[15rem_1fr]">
        <Sidebar />
        <div className="flex min-h-dvh min-w-0 flex-col pb-20 md:pb-0">
          <header className="bg-card flex h-14 items-center gap-3 border-b px-4 md:px-8">
            <span className="font-semibold tracking-tight md:hidden">
              RetireTui
            </span>
            <FileMenu />
            <DraftToolbar />
          </header>
          <main className="flex-1 px-4 py-6 md:px-8">
            <DraftNotices />
            {isNewPlan ? (
              <Outlet />
            ) : document ? (
              <>
                <GroupPages />
                <Outlet />
              </>
            ) : (
              <Start />
            )}
          </main>
        </div>
        <BottomBar />
      </div>
      <UnsavedQuestion />
    </FileActionsProvider>
  );
}
