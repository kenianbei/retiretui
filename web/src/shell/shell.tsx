import {
  Link,
  Outlet,
  useLocation,
  useMatches,
  useRouter,
} from "@tanstack/react-router";

import { Search } from "lucide-react";
import {
  useEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";

import { useComparedFollowDocument } from "@/compare/use-compared";
import { DraftNotices, UnsavedQuestion } from "@/draft/notices";
import { DraftToolbar } from "@/draft/toolbar";
import { FileActionsProvider } from "@/files/actions";
import { FileMenu } from "@/files/menu";
import { Start } from "@/files/start";
import { Button } from "@/components/ui/button";
import { GitHubMark } from "@/components/github-mark";
import { REPO_URL } from "@/links";
import { cn } from "@/lib/utils";
import { useSession } from "@/session";
import { Footer } from "@/shell/footer";
import { useTabTarget } from "@/shell/go";
import { KeysSheet } from "@/shell/keys";
import { useShellKeys } from "@/shell/use-keys";
import { Palette } from "@/shell/palette";
import {
  TABS,
  isGroup,
  isWithin,
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
  const target = useTabTarget();
  return (
    <Link
      {...target(tab, page)}
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
      className="bg-card hidden border-r px-3 py-4 md:sticky md:top-0 md:block md:h-dvh md:overflow-y-auto"
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
      className="bg-card fixed inset-x-0 bottom-0 z-10 border-t pr-[env(safe-area-inset-right)] pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)] md:hidden"
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
    link: "touch-target relative block rounded-full border px-3 py-1 text-sm whitespace-nowrap",
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

/** How far a row's fade reaches in from an edge with more beyond it. */
const FADE = "2rem";

/**
 * Whether a sideways row has more beyond its start and its end, the current
 * item scrolled into view whenever `shown` changes.
 */
function useScrollEdges(row: RefObject<HTMLElement | null>, shown: string) {
  const [isBefore, setBefore] = useState(false);
  const [isAfter, setAfter] = useState(false);
  useEffect(() => {
    const element = row.current;
    if (!element) return;
    const measure = () => {
      const { scrollLeft, clientWidth, scrollWidth } = element;
      setBefore(scrollLeft > 0);
      setAfter(scrollLeft + clientWidth < scrollWidth - 1);
    };
    measure();
    element.addEventListener("scroll", measure, { passive: true });
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => {
      element.removeEventListener("scroll", measure);
      observer.disconnect();
    };
  }, [row]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the current tab is scrolled to again when the page shown changes
  useEffect(() => {
    row.current
      ?.querySelector('[aria-current="page"]')
      ?.scrollIntoView({ inline: "nearest", block: "nearest" });
  }, [row, shown]);
  return { isBefore, isAfter };
}

/** A mask fading whichever edge of a row has more beyond it. */
function fadeOf({
  isBefore,
  isAfter,
}: {
  isBefore: boolean;
  isAfter: boolean;
}) {
  if (!isBefore && !isAfter) return undefined;
  const start = isBefore ? "transparent" : "black";
  const end = isAfter ? "transparent" : "black";
  return `linear-gradient(to right, ${start}, black ${FADE}, black calc(100% - ${FADE}), ${end})`;
}

/** The current group's pages above the page, where there is no sidebar. */
function GroupPages() {
  const isActive = useIsActive();
  const { pathname } = useLocation();
  const row = useRef<HTMLElement>(null);
  const edges = useScrollEdges(row, pathname);
  const tab = TABS.filter(isGroup).find(isActive);
  if (!tab) return null;
  return (
    <nav
      ref={row}
      aria-label={tab.title}
      style={{ maskImage: fadeOf(edges) }}
      className="-mx-4 -mt-2 mb-2 overflow-x-auto px-4 py-2 md:hidden"
    >
      <PageLinks tab={tab} variant="chips" />
    </nav>
  );
}

/**
 * Moves focus to the page's heading once a navigation to another page has
 * shown it, so that a screen reader says where it now is - unless the page
 * has put focus in itself, such as on the field a link names.
 */
function useFocusOnNavigation() {
  const router = useRouter();
  useEffect(() => {
    let before: Element | null = null;
    const leaving = router.subscribe("onBeforeNavigate", () => {
      before = window.document.activeElement;
    });
    const arrived = router.subscribe("onResolved", (event) => {
      if (!event.pathChanged || !event.fromLocation) return;
      requestAnimationFrame(() => {
        const main = window.document.querySelector("main");
        const heading = main?.querySelector("h1");
        const focused = window.document.activeElement;
        const isPlaced =
          focused !== before && focused !== main && main?.contains(focused);
        if (!heading || isPlaced) return;
        heading.tabIndex = -1;
        heading.focus();
      });
    });
    return () => {
      leaving();
      arrived();
    };
  }, [router]);
}

export function Shell() {
  const { document } = useSession();
  useComparedFollowDocument();
  useFocusOnNavigation();
  const main = useRef<HTMLElement>(null);
  const [isFinding, setFinding] = useState(false);
  const [isListingKeys, setListingKeys] = useState(false);
  useShellKeys(setFinding, setListingKeys);
  const isWithoutDocument = useMatches({
    select: (matches) =>
      matches.some((match) => match.staticData.isWithoutDocument === true),
  });
  return (
    <FileActionsProvider>
      <a
        href="#main"
        onClick={(event) => {
          event.preventDefault();
          main.current?.focus();
        }}
        className="bg-primary text-primary-foreground sr-only z-50 rounded-md px-3 py-2 focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
      >
        Skip to the page
      </a>
      <div className="min-h-dvh pr-[env(safe-area-inset-right)] pl-[env(safe-area-inset-left)] md:grid md:grid-cols-[12rem_1fr] lg:grid-cols-[15rem_1fr]">
        <Sidebar />
        <div className="flex min-h-dvh min-w-0 flex-col pb-[calc(5rem+env(safe-area-inset-bottom))] md:pb-0">
          <header className="bg-card flex h-14 items-center gap-2 border-b px-4 md:sticky md:top-0 md:z-40 md:gap-3 md:px-8">
            <FileMenu />
            <Button
              variant="outline"
              size="sm"
              aria-label="Find a page, a plan or an action"
              title="Find a page, a plan or an action (Ctrl K)"
              onClick={() => {
                setFinding(true);
              }}
            >
              <Search aria-hidden />
              <kbd className="text-muted-foreground hidden font-sans text-xs lg:inline">
                Ctrl K
              </kbd>
            </Button>
            <DraftToolbar />
            <Button
              asChild
              variant="ghost"
              size="icon"
              className="max-md:hidden"
            >
              <a
                href={REPO_URL}
                target="_blank"
                rel="noreferrer"
                aria-label="RetireTui on GitHub"
                title="RetireTui on GitHub"
              >
                <GitHubMark />
              </a>
            </Button>
          </header>
          <main
            id="main"
            ref={main}
            tabIndex={-1}
            className="@container/page flex-1 px-4 py-6 outline-none md:px-8"
          >
            <DraftNotices />
            {isWithoutDocument ? (
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
          <Footer />
        </div>
        <BottomBar />
      </div>
      <UnsavedQuestion />
      <Palette
        isOpen={isFinding}
        setOpen={setFinding}
        showKeys={() => {
          setListingKeys(true);
        }}
      />
      <KeysSheet isOpen={isListingKeys} setOpen={setListingKeys} />
    </FileActionsProvider>
  );
}
