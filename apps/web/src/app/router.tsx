import { lazy, Suspense, type ReactNode } from "react";
import { Navigate, createBrowserRouter, useLocation } from "react-router-dom";
import { useApp } from "./AppContext";
import { AppShell } from "../layouts/AppShell";

const LoginPage = lazy(() => import("../features/auth/LoginPage").then((module) => ({ default: module.LoginPage })));
const PortalPage = lazy(() => import("../features/portal/PortalPage").then((module) => ({ default: module.PortalPage })));
const ExecutionWorkspace = lazy(() => import("../features/execution/ExecutionWorkspace").then((module) => ({ default: module.ExecutionWorkspace })));
const FitnessPage = lazy(() => import("../features/fitness/FitnessPage").then((module) => ({ default: module.FitnessPage })));
const HealthPage = lazy(() => import("../features/health/HealthPage").then((module) => ({ default: module.HealthPage })));
const NotesPage = lazy(() => import("../features/notes/NotesPage").then((module) => ({ default: module.NotesPage })));
const MailPage = lazy(() => import("../features/mail/MailPage").then((module) => ({ default: module.MailPage })));
const FinanceWorkspace = lazy(() => import("../features/finance/FinanceWorkspace").then((module) => ({ default: module.FinanceWorkspace })));
const AssistantPage = lazy(() => import("../features/assistant/AssistantPage").then((module) => ({ default: module.AssistantPage })));
const SearchPage = lazy(() => import("../features/search/SearchPage").then((module) => ({ default: module.SearchPage })));
const SettingsPage = lazy(() => import("../features/settings/SettingsPage").then((module) => ({ default: module.SettingsPage })));
const UiShowcasePage = lazy(() => import("../features/system/UiShowcasePage").then((module) => ({ default: module.UiShowcasePage })));

function PageFallback() {
  return (
    <div className="page-shell">
      <div className="space-y-3" aria-label="页面加载中" aria-busy="true">
        <div className="h-8 w-40 animate-pulse rounded-md bg-muted" />
        <div className="h-4 w-72 max-w-full animate-pulse rounded bg-muted" />
        <div className="mt-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {Array.from({ length: 4 }, (_, index) => <div key={index} className="h-28 animate-pulse rounded-lg border bg-card" />)}
        </div>
      </div>
    </div>
  );
}

function withSuspense(element: ReactNode) {
  return <Suspense fallback={<PageFallback />}>{element}</Suspense>;
}

function RequireAuth({ children }: { children: ReactNode }) {
  const { session, authLoading } = useApp();
  const location = useLocation();

  if (authLoading) {
    return <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">正在验证 LifeTrace Cloud 会话…</div>;
  }

  if (!session) {
    const target = `${location.pathname}${location.search}`;
    return <Navigate to={`/login?redirect=${encodeURIComponent(target)}`} replace />;
  }

  return <>{children}</>;
}

function ProtectedShell() {
  return <RequireAuth><AppShell /></RequireAuth>;
}

function LegacyFinanceRedirect() {
  const location = useLocation();
  const pathname = location.pathname.replace(/^\/app\/finance/, "/finance");
  return <Navigate to={`${pathname || "/finance"}${location.search}${location.hash}`} replace />;
}

export const router = createBrowserRouter([
  { path: "/", element: <RequireAuth>{withSuspense(<PortalPage />)}</RequireAuth> },
  { path: "/login", element: withSuspense(<LoginPage />) },
  { path: "/notes/*", element: <RequireAuth>{withSuspense(<NotesPage />)}</RequireAuth> },
  { path: "/mail/*", element: <RequireAuth>{withSuspense(<MailPage />)}</RequireAuth> },
  { path: "/finance/*", element: <RequireAuth>{withSuspense(<FinanceWorkspace />)}</RequireAuth> },
  { path: "/execute", element: <Navigate to="/execute/today" replace /> },
  { path: "/execute/:view", element: <RequireAuth>{withSuspense(<ExecutionWorkspace />)}</RequireAuth> },
  { path: "/execute/*", element: <Navigate to="/execute/today" replace /> },
  {
    path: "/app",
    element: <ProtectedShell />,
    children: [
      { index: true, element: <Navigate to="health" replace /> },
      { path: "today", element: <Navigate to="/execute/today" replace /> },
      { path: "execution", element: <Navigate to="/execute/today" replace /> },
      { path: "calendar", element: <Navigate to="/execute/planner" replace /> },
      { path: "habits", element: <Navigate to="/execute/habits" replace /> },
      { path: "fitness", element: withSuspense(<FitnessPage />) },
      { path: "health", element: withSuspense(<HealthPage />) },
      { path: "notes", element: <Navigate to="/notes" replace /> },
      { path: "review", element: <Navigate to="/execute/review" replace /> },
      { path: "finance/*", element: <LegacyFinanceRedirect /> },
      { path: "assistant", element: withSuspense(<AssistantPage />) },
      { path: "search", element: withSuspense(<SearchPage />) },
      { path: "settings/*", element: withSuspense(<SettingsPage />) },
      { path: "system/ui", element: withSuspense(<UiShowcasePage />) },
      { path: "*", element: <Navigate to="health" replace /> },
    ],
  },
  { path: "*", element: <Navigate to="/" replace /> },
]);
