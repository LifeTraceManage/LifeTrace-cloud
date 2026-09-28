import { useState, type FormEvent } from "react";
import { Check, Flame, Plus, Trash2 } from "lucide-react";
import { useApp } from "../../app/AppContext";
import {
  AlertDialog, Badge, Button, Card, CardContent, Dialog, EmptyState, Input, PageHeader, Progress, Select, Textarea, cn,
} from "../../components/ui";
import { entities, recentDays, text, todayKey } from "../../lib/entities";
import { createHabitActivity, createHabitLog, type JsonEntity } from "../../services/core";

const WEEKDAYS = [
  { value: 1, label: "一" },
  { value: 2, label: "二" },
  { value: 3, label: "三" },
  { value: 4, label: "四" },
  { value: 5, label: "五" },
  { value: 6, label: "六" },
  { value: 7, label: "日" },
] as const;

function weekday(dateKey: string): number {
  const date = new Date(`${dateKey}T12:00:00`);
  const day = date.getDay();
  return day === 0 ? 7 : day;
}

export function habitScheduledOnDate(activity: JsonEntity, dateKey: string): boolean {
  const startDate = text(activity, "startDate");
  if (startDate && dateKey < startDate) return false;
  const targetDays = Array.isArray(activity.targetDays)
    ? activity.targetDays.map(Number).filter((day) => day >= 1 && day <= 7)
    : [];
  if (targetDays.length) return targetDays.includes(weekday(dateKey));
  return true;
}

function completedDatesFor(activityId: string, logs: JsonEntity[]): Set<string> {
  return new Set(
    logs
      .filter((item) => item.activityId === activityId && text(item, "status") === "completed")
      .map((item) => text(item, "logDate")),
  );
}

function currentStreak(activity: JsonEntity, completedDates: Set<string>): number {
  const cursor = new Date(`${todayKey()}T12:00:00`);
  let streak = 0;
  let checked = 0;
  while (checked < 366) {
    const key = `${cursor.getFullYear()}-${String(cursor.getMonth() + 1).padStart(2, "0")}-${String(cursor.getDate()).padStart(2, "0")}`;
    if (habitScheduledOnDate(activity, key)) {
      if (!completedDates.has(key)) break;
      streak += 1;
    }
    cursor.setDate(cursor.getDate() - 1);
    checked += 1;
  }
  return streak;
}

function targetLabel(activity: JsonEntity): string {
  const target = Number(activity.normalTarget ?? 1);
  const unit = text(activity, "unit", "次");
  return `${Number.isFinite(target) ? target : 1} ${unit}`;
}

function scheduleLabel(activity: JsonEntity): string {
  const targetDays = Array.isArray(activity.targetDays) ? activity.targetDays.map(Number) : [];
  if (!targetDays.length) return "每天";
  const labels = WEEKDAYS.filter((day) => targetDays.includes(day.value)).map((day) => `周${day.label}`);
  return labels.join("、");
}

export function HabitsPanel({ embedded = false }: { embedded?: boolean }) {
  const { state, session, upsert, remove } = useApp();
  const [showNew, setShowNew] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<JsonEntity | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [name, setName] = useState("");
  const [activityType, setActivityType] = useState("completion");
  const [normalTarget, setNormalTarget] = useState("1");
  const [minimumTarget, setMinimumTarget] = useState("");
  const [unit, setUnit] = useState("次");
  const [scheduleType, setScheduleType] = useState<"daily" | "custom">("daily");
  const [targetDays, setTargetDays] = useState<number[]>([]);
  const [startDate, setStartDate] = useState(todayKey());
  const [description, setDescription] = useState("");

  const activities = entities(state, "habit.activity").filter((item) => !item.isArchived);
  const logs = entities(state, "habit.log");
  const today = todayKey();
  const days7 = recentDays(7);
  const days30 = recentDays(30);

  function resetForm() {
    setName("");
    setActivityType("completion");
    setNormalTarget("1");
    setMinimumTarget("");
    setUnit("次");
    setScheduleType("daily");
    setTargetDays([]);
    setStartDate(todayKey());
    setDescription("");
  }

  function changeActivityType(next: string) {
    setActivityType(next);
    if (next === "duration") {
      setUnit("分钟");
      if (normalTarget === "1") setNormalTarget("30");
    } else if (next === "count") {
      setUnit("次");
    } else {
      setUnit("次");
      setNormalTarget("1");
      setMinimumTarget("");
    }
  }

  function toggleTargetDay(day: number) {
    setTargetDays((current) =>
      current.includes(day) ? current.filter((item) => item !== day) : [...current, day].sort(),
    );
  }

  async function add(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    if (scheduleType === "custom" && targetDays.length === 0) return;

    const target = Math.max(0.01, Number(normalTarget) || 1);
    const minimum = minimumTarget.trim() ? Math.max(0, Number(minimumTarget) || 0) : null;
    await upsert("habit.activity", createHabitActivity(session.user.id, session.session.deviceId, {
      name,
      activityType,
      unit,
      minimumTarget: minimum,
      normalTarget: target,
      targetPeriod: "daily",
      targetDays: scheduleType === "custom" ? targetDays : [],
      scheduleType,
      startDate,
      checkinMethod: "manual",
      description,
    }));
    resetForm();
    setShowNew(false);
  }

  async function deleteHabit(activity: JsonEntity) {
    if (deleting) return;
    setDeleting(true);
    try {
      const relatedLogs = logs.filter((item) => item.activityId === activity.meta.id);
      for (const item of relatedLogs) {
        await remove("habit.log", item.meta.id);
      }
      await remove("habit.activity", activity.meta.id);
      setDeleteTarget(null);
    } finally {
      setDeleting(false);
    }
  }

  async function toggle(activity: JsonEntity) {
    const existing = logs.find((item) =>
      item.activityId === activity.meta.id
      && text(item, "logDate") === today
      && text(item, "status") === "completed"
    );
    if (existing) {
      await remove("habit.log", existing.meta.id);
      return;
    }
    if (!session) return;
    const target = Number(activity.normalTarget ?? 1);
    await upsert(
      "habit.log",
      createHabitLog(
        session.user.id,
        session.session.deviceId,
        activity.meta.id,
        Number.isFinite(target) ? target : 1,
        "",
        today,
      ),
    );
  }

  const content = <>
    <PageHeader
      title={embedded ? "习惯" : "坚持"}
      description={embedded ? "习惯与计划任务统一归入 Execute；频率和目标会直接影响 Today 与 Review。" : undefined}
      action={<Button onClick={() => setShowNew(true)}><Plus size={16} />新建习惯</Button>}
    />

    <Dialog
      open={showNew}
      onOpenChange={(open) => {
        setShowNew(open);
        if (!open) resetForm();
      }}
      title="新建习惯"
      description="配置目标、频率和执行日；这些设置会直接影响 Execute Today 与 Review。"
      className="max-w-3xl"
    >
      <form className="space-y-5" onSubmit={(event) => void add(event)}>
          <div className="grid gap-4 md:grid-cols-2">
            <label className="space-y-1.5 text-sm">
              <span className="font-medium">习惯名称</span>
              <Input autoFocus value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：阅读" required />
            </label>
            <label className="space-y-1.5 text-sm">
              <span className="font-medium">记录类型</span>
              <Select value={activityType} onChange={(event) => changeActivityType(event.target.value)}>
                <option value="completion">完成 / 未完成</option>
                <option value="count">次数 / 数量</option>
                <option value="duration">时长</option>
              </Select>
            </label>
          </div>

          <div className="grid gap-4 md:grid-cols-3">
            <label className="space-y-1.5 text-sm">
              <span className="font-medium">正常目标</span>
              <Input type="number" min="0.01" step="0.01" value={normalTarget} onChange={(event) => setNormalTarget(event.target.value)} required />
            </label>
            <label className="space-y-1.5 text-sm">
              <span className="font-medium">单位</span>
              <Input value={unit} onChange={(event) => setUnit(event.target.value)} placeholder="次 / 分钟 / 页" required />
            </label>
            <label className="space-y-1.5 text-sm">
              <span className="font-medium">最低目标 <span className="text-muted-foreground">可选</span></span>
              <Input type="number" min="0" step="0.01" value={minimumTarget} onChange={(event) => setMinimumTarget(event.target.value)} placeholder="例如 10" disabled={activityType === "completion"} />
            </label>
          </div>

          <div className="grid gap-4 md:grid-cols-[180px_minmax(0,1fr)_170px]">
            <label className="space-y-1.5 text-sm">
              <span className="font-medium">执行频率</span>
              <Select value={scheduleType} onChange={(event) => {
                const next = event.target.value as "daily" | "custom";
                setScheduleType(next);
                if (next === "daily") setTargetDays([]);
              }}>
                <option value="daily">每天</option>
                <option value="custom">指定星期</option>
              </Select>
            </label>

            <div className="space-y-1.5 text-sm">
              <span className="font-medium">执行日</span>
              <div className="flex min-h-10 flex-wrap items-center gap-2">
                {scheduleType === "daily"
                  ? <span className="text-sm text-muted-foreground">每天执行</span>
                  : WEEKDAYS.map((day) => <button
                    key={day.value}
                    type="button"
                    onClick={() => toggleTargetDay(day.value)}
                    className={cn(
                      "flex h-9 w-9 shrink-0 items-center justify-center rounded-md border text-xs",
                      targetDays.includes(day.value) ? "border-primary bg-primary text-primary-foreground" : "bg-background hover:bg-muted",
                    )}
                  >{day.label}</button>)}
              </div>
              {scheduleType === "custom" && targetDays.length === 0
                ? <div className="text-xs text-destructive">至少选择一个执行日</div>
                : null}
            </div>

            <label className="space-y-1.5 text-sm">
              <span className="font-medium">开始日期</span>
              <Input type="date" value={startDate} onChange={(event) => setStartDate(event.target.value)} required />
            </label>
          </div>

          <label className="block space-y-1.5 text-sm">
            <span className="font-medium">说明 <span className="text-muted-foreground">可选</span></span>
            <Textarea value={description} onChange={(event) => setDescription(event.target.value)} placeholder="为什么要做、完成标准或其他备注…" className="min-h-20" />
          </label>

        <div className="flex flex-wrap justify-end gap-2 border-t pt-4">
          <Button variant="ghost" onClick={() => { resetForm(); setShowNew(false); }}>取消</Button>
          <Button type="submit" disabled={!name.trim() || (scheduleType === "custom" && targetDays.length === 0)}>创建习惯</Button>
        </div>
      </form>
    </Dialog>

    {activities.length ? <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{activities.map((activity) => {
      const completedDates = completedDatesFor(activity.meta.id, logs);
      const completedToday = completedDates.has(today);
      const scheduled7 = days7.filter((day) => habitScheduledOnDate(activity, day));
      const scheduled30 = days30.filter((day) => habitScheduledOnDate(activity, day));
      const weekCount = scheduled7.filter((day) => completedDates.has(day)).length;
      const monthCount = scheduled30.filter((day) => completedDates.has(day)).length;
      const weekPercent = scheduled7.length ? Math.round(weekCount / scheduled7.length * 100) : 0;
      const monthPercent = scheduled30.length ? Math.round(monthCount / scheduled30.length * 100) : 0;
      const streak = currentStreak(activity, completedDates);
      const dueToday = habitScheduledOnDate(activity, today);

      return <Card key={activity.meta.id}><CardContent className="pt-5">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <div className="truncate text-base font-semibold">{text(activity, "name", "习惯")}</div>
              {streak > 0 ? <Badge className="border-warning/25 bg-warning/10 text-warning"><Flame size={12} className="mr-1" />{streak} 次 streak</Badge> : null}
            </div>
            <div className="mt-1 text-xs text-muted-foreground">{scheduleLabel(activity)} · 目标 {targetLabel(activity)}</div>
            {text(activity, "description") ? <p className="mt-2 line-clamp-2 text-xs leading-5 text-muted-foreground">{text(activity, "description")}</p> : null}
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <Button
              size="icon"
              variant="ghost"
              className="text-muted-foreground hover:text-destructive"
              onClick={() => setDeleteTarget(activity)}
              aria-label={`删除习惯 ${text(activity, "name", "习惯")}`}
              title="删除习惯"
            ><Trash2 size={16} /></Button>
            <button
              onClick={() => void toggle(activity)}
              disabled={!dueToday}
              className={cn(
                "flex h-10 w-10 shrink-0 items-center justify-center rounded-full border",
                completedToday && "border-primary bg-primary text-primary-foreground",
                !dueToday && "cursor-not-allowed opacity-35",
              )}
              aria-label={!dueToday ? "今天无需执行" : completedToday ? "取消今日打卡" : "今日打卡"}
            >{completedToday ? <Check size={18} /> : <Flame size={17} />}</button>
          </div>
        </div>

        <div className="mt-5 space-y-3">
          <div><div className="mb-1.5 flex justify-between text-xs"><span>近 7 天完成率</span><span className="text-muted-foreground">{weekCount}/{scheduled7.length} · {weekPercent}%</span></div><Progress value={weekPercent} /></div>
          <div><div className="mb-1.5 flex justify-between text-xs"><span>近 30 天完成率</span><span className="text-muted-foreground">{monthCount}/{scheduled30.length} · {monthPercent}%</span></div><Progress value={monthPercent} /></div>
        </div>

        <div className="mt-5">
          <div className="mb-2 flex items-center justify-between text-[11px] text-muted-foreground"><span>30 天 Heatmap</span><span>{dueToday ? "今日应执行" : "今日休息"}</span></div>
          <div className="grid grid-cols-10 gap-1">{days30.map((day) => {
            const scheduled = habitScheduledOnDate(activity, day);
            return <div
              key={day}
              title={`${day}${scheduled ? " · 应执行" : " · 休息"}${completedDates.has(day) ? " · 已完成" : ""}`}
              className={cn("aspect-square min-h-3 rounded-sm", completedDates.has(day) ? "bg-primary" : scheduled ? "bg-muted" : "bg-muted/30")}
            />;
          })}</div>
        </div>
      </CardContent></Card>;
    })}</div> : <EmptyState title="还没有习惯" action={<Button variant="outline" onClick={() => setShowNew(true)}>创建第一个习惯</Button>} />}

    <AlertDialog
      open={Boolean(deleteTarget)}
      onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
      title="删除习惯"
      description={deleteTarget
        ? `确定删除“${text(deleteTarget, "name", "习惯")}”吗？该习惯的历史打卡记录也会一并删除，此操作会同步到云端。`
        : undefined}
    >
      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={() => setDeleteTarget(null)}>取消</Button>
        <Button variant="destructive" disabled={deleting} onClick={() => { if (deleteTarget) void deleteHabit(deleteTarget); }}>
          <Trash2 size={15} />{deleting ? "删除中…" : "删除习惯"}
        </Button>
      </div>
    </AlertDialog>
  </>;

  return embedded ? <div>{content}</div> : <div className="page-shell">{content}</div>;
}

export function HabitsPage() {
  return <HabitsPanel />;
}
