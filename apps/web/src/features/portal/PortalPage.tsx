import { Link } from "react-router-dom";
import { Home, ArrowRight, Leaf, Mail, NotebookPen } from "lucide-react";
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
    to: "/app/today",
    name: "Execute",
    description: "任务、日历与日常执行。",
    icon: Home,
    accent: "text-warning",
  },
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
    </div>
  </main>;
}
