import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  Outlet,
  useRouterState,
} from "@tanstack/react-router";
import { History, LayoutDashboard, Settings, Tags, Users } from "lucide-react";
import type { ReactNode } from "react";

import { BillDetailPage } from "@/pages/bill-detail";
import { BillTypesPage } from "@/pages/bill-types";
import { BoarderDetailPage } from "@/pages/boarder-detail";
import { BoardersPage } from "@/pages/boarders";
import { DashboardPage } from "@/pages/dashboard";
import { HistoryBillPage } from "@/pages/history-detail";
import { HistoryPage } from "@/pages/history";
import { NewBillPage } from "@/pages/new-bill";
import { NewPaymentPage } from "@/pages/new-payment";
import { SettingsPage } from "@/pages/settings";
import { cn } from "@/components/ui";

const NAV_ITEMS: Array<{ to: string; label: string; icon: ReactNode }> = [
  { to: "/", label: "Dashboard", icon: <LayoutDashboard size={17} /> },
  { to: "/history", label: "History", icon: <History size={17} /> },
  { to: "/boarders", label: "Boarders", icon: <Users size={17} /> },
  { to: "/bill-types", label: "Bill types", icon: <Tags size={17} /> },
  { to: "/settings", label: "Settings", icon: <Settings size={17} /> },
];

function AppShell() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });

  return (
    <div className="flex h-full">
      <aside className="flex w-56 shrink-0 flex-col border-r border-line bg-panel px-3 py-5">
        <div className="px-2">
          <p className="text-[11px] font-semibold tracking-widest text-muted uppercase">
            Boarding house
          </p>
          <p className="text-base font-bold tracking-tight text-ink">Treasurer</p>
        </div>

        <nav className="mt-6 flex flex-col gap-1">
          {NAV_ITEMS.map((item) => {
            const active =
              item.to === "/" ? pathname === "/" : pathname === item.to || pathname.startsWith(`${item.to}/`);
            return (
              <Link
                key={item.to}
                to={item.to}
                className={cn(
                  "flex items-center gap-2.5 rounded-xl px-3 py-2 text-sm font-medium transition-colors",
                  active ? "bg-accent-soft text-accent" : "text-muted hover:bg-canvas hover:text-ink",
                )}
              >
                {item.icon}
                {item.label}
              </Link>
            );
          })}
        </nav>

        <div className="mt-auto rounded-xl bg-canvas p-3 text-[11px] leading-relaxed text-muted">
          Desktop edition. Data stays on this computer in the app data folder.
        </div>
      </aside>

      <main className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-6xl px-8 py-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}

const rootRoute = createRootRoute({ component: AppShell });

function numberParam(value: unknown, fallback: number): number {
  const parsed = typeof value === "number" ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

const dashboardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: DashboardPage,
});

const boardersRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/boarders",
  component: BoardersPage,
});

const historyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/history",
  component: HistoryPage,
});

const historyDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/history/$id",
  component: HistoryBillPage,
});

const boarderDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/boarders/$id",
  component: BoarderDetailPage,
});

const billTypesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/bill-types",
  component: BillTypesPage,
});

const newBillRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/bills/new",
  validateSearch: (search: Record<string, unknown>) => ({
    year: search.year === undefined ? undefined : numberParam(search.year, 0),
    month: search.month === undefined ? undefined : numberParam(search.month, 0),
  }),
  component: NewBillPage,
});

const billDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/bills/$id",
  component: BillDetailPage,
});

const newPaymentRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/payments/new",
  validateSearch: (search: Record<string, unknown>) => ({
    shareId: typeof search.shareId === "string" ? search.shareId : undefined,
  }),
  component: NewPaymentPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
});

const routeTree = rootRoute.addChildren([
  dashboardRoute,
  historyRoute,
  historyDetailRoute,
  boardersRoute,
  boarderDetailRoute,
  billTypesRoute,
  newBillRoute,
  billDetailRoute,
  newPaymentRoute,
  settingsRoute,
]);

export const router = createRouter({
  routeTree,
  history: createHashHistory(),
  defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
