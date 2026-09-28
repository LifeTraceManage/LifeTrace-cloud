import { Link } from "react-router-dom";
import { Activity, ArrowRight, Bot, Dumbbell, HeartPulse, LayoutDashboard, Leaf, Mail, NotebookPen, Settings, WalletCards } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Card, CardContent, cn } from "../../components/ui";

const modules = [
  {
    to: "/notes",
    name: "Notes",
    description: "笔记与知识管理。",
    icon: NotebookPen,
    accent: "text-primary",
  },
  {
    to: "/mail",
    name: "Mail",
    description: "收发、搜索与管理邮件。",
    icon: Mail,
    accent: "text-info",
  },
  {
    to: "/execute/today",
    name: "Execute",
    description: "任务、日历与日常执行。",
    icon: LayoutDashboard,
    accent: "text-warning",
  },
] as const;

const coreModules = [
  { to: "/app/health", name: "Health", icon: HeartPulse },
  { to: "/app/fitness", name: "Fitness", icon: Dumbbell },
  { to: "/app/habits", name: "Habits", icon: Activity },
  { to: "/app/finance", name: "Finance", icon: WalletCards },
  { to: "/app/assistant", name: "Assistant", icon: Bot },
  { to: "/app/settings", name: "Settings", icon: Settings },
] as const;

export function PortalPage() {
  const { session } = useApp();

  return <main className="min-h-screen bg-background">
    <div className="mx-auto w-full max-w-6xl px-4 py-8 sm:px-6 lg:px-8 lg:py-12">
      <header className="mb-10 flex flex-col gap-6 border-b pb-8 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <div className="mb-4 flex items-center gap-3">
            <span className="flex h-11 w-11 items-center justify-center rounded-xl bg-primary text-primary-foreground"><Leaf size={20} /></span>
            <div>
              <div className="text-lg font-semibold tracking-[-0.025em]">LifeTrace</div>
              </div>
          </div>
          <h1 className="max-w-2xl text-3xl font-semibold tracking-[-0.035em] sm:text-4xl">一个账号，进入不同工作区。</h1>
        </div>
        <div className="rounded-lg border bg-card px-4 py-3 text-sm">
          <div className="text-xs text-muted-foreground">当前账号</div>
          <div className="mt-1 font-medium">{session?.user.displayName || session?.user.email || "LifeTrace User"}</div>
        </div>
      </header>

      <div className="mb-4 flex items-center justify-between">
        <div>
          <div className="eyebrow">Workspaces</div>
          <h2 className="mt-1 text-lg font-semibold">选择工作区</h2>
        </div>
      </div>

      <section className="grid gap-4 md:grid-cols-3">
        {modules.map(({ to, name, description, icon: Icon, accent }) => <Link key={to} to={to} className="group block">
          <Card className="h-full transition-colors group-hover:border-primary/35 group-hover:bg-accent/25">
            <CardContent className="flex h-full flex-col pt-5">
              <span className={cn("flex h-10 w-10 items-center justify-center rounded-lg bg-muted", accent)}><Icon size={19} /></span>
              <div className="mt-5 text-lg font-semibold tracking-[-0.02em]">{name}</div>
              <p className="mt-2 flex-1 text-sm leading-6 text-muted-foreground">{description}</p>
              <div className="mt-6 flex items-center gap-2 text-sm font-medium text-primary">打开 {name}<ArrowRight size={15} className="transition-transform group-hover:translate-x-0.5" /></div>
            </CardContent>
          </Card>
        </Link>)}
      </section>

      <section className="mt-10 border-t pt-7">
        <div className="eyebrow">LifeTrace Core</div>
        <h2 className="mt-1 text-lg font-semibold">其他功能</h2>
        <p className="mt-1 text-sm text-muted-foreground">这些功能继续由 LifeTrace Core 承载，功能成熟后再独立为工作区。</p>
        <div className="mt-4 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">{coreModules.map(({to,name,icon:Icon})=><Link key={to} to={to} className="flex items-center gap-3 rounded-lg border bg-card px-4 py-3 text-sm font-medium transition-colors hover:bg-muted"><Icon size={16} className="text-muted-foreground"/><span className="flex-1">{name}</span><ArrowRight size={14} className="text-muted-foreground"/></Link>)}</div>
      </section>
    </div>
  </main>;
}
