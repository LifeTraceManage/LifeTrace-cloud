import type { ReactNode } from "react";
import { Link, NavLink } from "react-router-dom";
import { Home, Leaf, LogOut, Mail, Moon, NotebookPen, Sun } from "lucide-react";
import { useApp } from "../app/AppContext";
import { Badge, Button, cn } from "../components/ui";

const workspaceLinks = [
  { to: "/notes", label: "Notes", icon: NotebookPen },
  { to: "/mail", label: "Mail", icon: Mail },
  { to: "/app/today", label: "Execute", icon: Home },
] as const;

export function WorkspaceShell({
  title,
  description,
  icon,
  action,
  children,
}: {
  title: string;
  description?: string;
  icon: ReactNode;
  action?: ReactNode;
  children: ReactNode;
}) {
  const { session, online, loading, theme, setTheme, logout, error, clearError } = useApp();

  return <div className="min-h-screen bg-background">
    <header className="sticky top-0 z-40 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/85">
      <div className="flex h-14 items-center gap-3 px-3 sm:px-5 lg:h-16 lg:px-6">
        <Link to="/" className="flex shrink-0 items-center gap-2 rounded-md px-1 py-1 text-sm font-semibold tracking-[-0.02em] hover:text-primary">
          <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground"><Leaf size={16} /></span>
          <span className="hidden sm:inline">LifeTrace</span>
        </Link>

        <div className="hidden h-6 w-px bg-border sm:block" />

        <div className="flex min-w-0 items-center gap-2">
          <span className="text-primary">{icon}</span>
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold">{title}</div>
            {description ? <div className="hidden truncate text-[11px] text-muted-foreground md:block">{description}</div> : null}
          </div>
        </div>

        <nav className="ml-auto hidden items-center gap-1 lg:flex" aria-label="LifeTrace 工作区">
          {workspaceLinks.map(({ to, label, icon: Icon }) => <NavLink key={to} to={to} className={({ isActive }) => cn("flex h-9 items-center gap-2 rounded-md px-3 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground", isActive && "bg-accent font-medium text-accent-foreground")}><Icon size={15} />{label}</NavLink>)}
        </nav>

        <div className="ml-auto flex items-center gap-1 lg:ml-2">
          {!online ? <Badge className="border-warning/30 bg-warning/10 text-warning">离线</Badge> : null}
          {loading ? <Badge>同步中</Badge> : null}
          {action}
          <Button size="icon" variant="ghost" onClick={() => void setTheme(theme === "dark" ? "light" : "dark")} aria-label="切换主题">
            {theme === "dark" ? <Sun size={17} /> : <Moon size={17} />}
          </Button>
          <div className="hidden items-center gap-2 border-l pl-2 sm:flex">
            <div className="flex h-8 w-8 items-center justify-center rounded-full border bg-card text-[11px] font-semibold">
              {(session?.user.displayName || session?.user.email || "LT").slice(0, 2).toUpperCase()}
            </div>
            <Button size="icon" variant="ghost" onClick={() => void logout()} aria-label="退出登录"><LogOut size={16} /></Button>
          </div>
        </div>
      </div>

      <nav className="flex h-10 items-center gap-1 overflow-x-auto border-t px-3 lg:hidden" aria-label="LifeTrace 工作区">
        {workspaceLinks.map(({ to, label, icon: Icon }) => <NavLink key={to} to={to} className={({ isActive }) => cn("flex h-8 shrink-0 items-center gap-2 rounded-md px-2.5 text-xs text-muted-foreground", isActive && "bg-accent font-medium text-accent-foreground")}><Icon size={14} />{label}</NavLink>)}
      </nav>
    </header>

    {error ? <div className="mx-3 mt-3 flex items-start justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive sm:mx-5 lg:mx-6"><span>{error}</span><button onClick={clearError}>关闭</button></div> : null}

    {children}
  </div>;
}
