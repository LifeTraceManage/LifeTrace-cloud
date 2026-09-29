import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Bot, Check, Plus, Send, X } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, PageHeader, Textarea } from "../../components/ui";
import { AssistantApi, type AssistantApproval, type AssistantSession } from "../../services/core";

type Message = { role: "user" | "assistant"; content: string; provider?: string };

function approvalLabel(approval: AssistantApproval): string {
  if (approval.actionName === "create_task") return "创建任务";
  if (approval.actionName === "update_task") return "修改任务";
  if (approval.actionName === "create_calendar_event") return "创建日程";
  if (approval.actionName === "create_project") return "创建 Project";
  if (approval.actionName === "update_project") return "修改 Project";
  if (approval.actionName === "create_habit") return "创建习惯";
  if (approval.actionName === "update_habit") return "修改习惯";
  return approval.actionName;
}

function approvalSummary(approval: AssistantApproval): string {
  const action = approval.actionJson;
  const title = typeof action.title === "string" ? action.title : "";
  if (approval.actionName === "create_task") {
    const due = typeof action.dueAt === "string" ? ` · 截止 ${new Date(action.dueAt).toLocaleString()}` : "";
    return `${title || "未命名任务"}${due}`;
  }
  if (approval.actionName === "update_task") {
    const parts = [
      typeof action.title === "string" ? `标题 → ${action.title}` : "",
      typeof action.status === "string" ? `状态 → ${action.status}` : "",
      typeof action.priority === "string" ? `优先级 → ${action.priority}` : "",
      typeof action.dueAt === "string" ? `截止 → ${new Date(action.dueAt).toLocaleString()}` : "",
      action.clearDueAt === true ? "清除截止时间" : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : `任务 ${String(action.taskId ?? "")}`;
  }
  if (approval.actionName === "create_calendar_event") {
    const when =
      typeof action.startAt === "string"
        ? new Date(action.startAt).toLocaleString()
        : typeof action.startLocalDate === "string"
          ? action.startLocalDate
          : "";
    return `${title || "未命名日程"}${when ? ` · ${when}` : ""}`;
  }
  if (approval.actionName === "create_project") {
    return typeof action.name === "string" ? action.name : "未命名 Project";
  }
  if (approval.actionName === "update_project") {
    const parts = [
      typeof action.name === "string" ? `名称 → ${action.name}` : "",
      typeof action.status === "string" ? `状态 → ${action.status}` : "",
      action.clearDescription === true ? "清除说明" : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : `Project ${String(action.projectId ?? "")}`;
  }
  if (approval.actionName === "create_habit") {
    const schedule = action.scheduleType === "custom" && Array.isArray(action.targetDays)
      ? `周 ${action.targetDays.join("、")}`
      : "每天";
    const target = typeof action.normalTarget === "number"
      ? ` · 目标 ${action.normalTarget} ${String(action.unit ?? "")}`
      : "";
    return `${String(action.name ?? "未命名习惯")} · ${schedule}${target}`;
  }
  if (approval.actionName === "update_habit") {
    const parts = [
      typeof action.name === "string" ? `名称 → ${action.name}` : "",
      typeof action.normalTarget === "number" ? `目标 → ${action.normalTarget}` : "",
      typeof action.scheduleType === "string" ? `频率 → ${action.scheduleType}` : "",
      action.isArchived === true ? "归档" : action.isArchived === false ? "取消归档" : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : `习惯 ${String(action.habitId ?? "")}`;
  }
  return "待确认写操作";
}

function statusLabel(status: string): string {
  if (status === "pending") return "待确认";
  if (status === "approved") return "已执行";
  if (status === "rejected") return "已拒绝";
  if (status === "expired") return "已过期";
  if (status === "cancelled") return "已取消";
  return status;
}

export function AssistantPage() {
  const { session } = useApp();
  const api = useMemo(() => new AssistantApi(), []);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessions, setSessions] = useState<AssistantSession[]>([]);
  const [prompt, setPrompt] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [approvals, setApprovals] = useState<AssistantApproval[]>([]);
  const [asking, setAsking] = useState(false);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [decidingApprovalId, setDecidingApprovalId] = useState<string | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!session) return;
    let active = true;
    void api
      .listSessions()
      .then((items) => {
        if (active) setSessions(items);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : "会话加载失败");
      });
    return () => {
      active = false;
    };
  }, [api, session]);

  async function refreshSessions() {
    if (!session) return;
    try {
      setSessions(await api.listSessions());
    } catch {
      // Conversation itself is still usable if history refresh fails.
    }
  }

  async function openSession(id: string) {
    if (asking || loadingHistory) return;
    setLoadingHistory(true);
    setError("");
    try {
      const [items, approvalItems] = await Promise.all([
        api.listMessages(id),
        api.listApprovals(id),
      ]);
      setSessionId(id);
      setApprovals(approvalItems);
      setMessages(
        items
          .filter((item) => item.role === "user" || item.role === "assistant")
          .map((item) => ({
            role: item.role as "user" | "assistant",
            content: item.content,
            provider: item.role === "assistant" ? item.provider ?? undefined : undefined,
          })),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "会话加载失败");
    } finally {
      setLoadingHistory(false);
    }
  }

  function newConversation() {
    if (asking) return;
    setSessionId(null);
    setMessages([]);
    setApprovals([]);
    setPrompt("");
    setError("");
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!session || !prompt.trim()) return;
    const text = prompt.trim();
    setPrompt("");
    setMessages((value) => [...value, { role: "user", content: text }]);
    setAsking(true);
    setError("");
    try {
      const reply = await api.ask(text, session.csrfToken, sessionId);
      setSessionId(reply.sessionId);
      setMessages((value) => [
        ...value,
        {
          role: "assistant",
          content: reply.reply,
          provider: reply.model ? `${reply.provider} · ${reply.model}` : reply.provider,
        },
      ]);
      const approvalItems = await api.listApprovals(reply.sessionId);
      setApprovals(approvalItems);
      await refreshSessions();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "AI 请求失败");
    } finally {
      setAsking(false);
    }
  }

  async function decideApproval(approvalId: string, decision: "approve" | "reject") {
    if (!session || decidingApprovalId) return;
    setDecidingApprovalId(approvalId);
    setError("");
    try {
      const result = await api.decideApproval(approvalId, decision, session.csrfToken);
      setApprovals((items) =>
        items.map((item) => (item.id === approvalId ? result.approval : item)),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "审批操作失败");
      if (sessionId) {
        try {
          setApprovals(await api.listApprovals(sessionId));
        } catch {
          // Keep the existing approval card if refresh also fails.
        }
      }
    } finally {
      setDecidingApprovalId(null);
    }
  }

  return (
    <div className="page-shell">
      <PageHeader title="AI 助手" />
      <div className="mx-auto grid max-w-6xl gap-4 lg:grid-cols-[260px_minmax(0,1fr)]">
        <Card className="h-fit">
          <CardContent className="space-y-3 pt-5">
            <Button className="w-full justify-start" variant="outline" onClick={newConversation} disabled={asking}>
              <Plus size={16} />
              新对话
            </Button>
            <div className="space-y-1">
              {sessions.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => void openSession(item.id)}
                  disabled={asking || loadingHistory}
                  className={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-muted ${
                    item.id === sessionId ? "bg-muted font-medium" : ""
                  }`}
                >
                  <div className="truncate">{item.title}</div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {new Date(item.lastMessageAt ?? item.updatedAt).toLocaleString()}
                  </div>
                </button>
              ))}
              {!sessions.length ? (
                <div className="px-2 py-3 text-xs text-muted-foreground">暂无历史会话</div>
              ) : null}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-5">
            <div className="min-h-[520px] space-y-4">
              {loadingHistory ? (
                <div className="text-sm text-muted-foreground">正在加载会话…</div>
              ) : !messages.length ? (
                <EmptyState
                  icon={<Bot size={26} />}
                  title="问问 LifeTrace"
                  description="可以查询已同步数据；创建/修改任务和创建日程会先生成审批，只有你确认后才执行。"
                />
              ) : (
                messages.map((message, index) => (
                  <div
                    key={index}
                    className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
                  >
                    <div
                      className={`max-w-[85%] rounded-lg px-4 py-3 text-sm leading-6 ${
                        message.role === "user" ? "bg-primary text-primary-foreground" : "bg-muted"
                      }`}
                    >
                      <div className="whitespace-pre-wrap">{message.content}</div>
                      {message.provider ? <Badge className="mt-2">{message.provider}</Badge> : null}
                    </div>
                  </div>
                ))
              )}
              {approvals.length ? (
                <div className="space-y-3 border-t pt-4">
                  {approvals.map((approval) => (
                    <div key={approval.id} className="rounded-lg border bg-card px-4 py-3">
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <div>
                          <div className="text-sm font-medium">{approvalLabel(approval)}</div>
                          <div className="mt-1 text-xs text-muted-foreground">{approvalSummary(approval)}</div>
                        </div>
                        <Badge>{statusLabel(approval.status)}</Badge>
                      </div>
                      {approval.status === "pending" ? (
                        <div className="mt-3 flex gap-2">
                          <Button
                            size="sm"
                            onClick={() => void decideApproval(approval.id, "approve")}
                            disabled={Boolean(decidingApprovalId)}
                          >
                            <Check size={14} />
                            批准并执行
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={() => void decideApproval(approval.id, "reject")}
                            disabled={Boolean(decidingApprovalId)}
                          >
                            <X size={14} />
                            拒绝
                          </Button>
                        </div>
                      ) : null}
                    </div>
                  ))}
                </div>
              ) : null}
              {asking ? <div className="text-sm text-muted-foreground">正在查询并分析云端记录…</div> : null}
              {error ? <div className="text-sm text-destructive">{error}</div> : null}
            </div>
            <form className="mt-4 flex items-end gap-2 border-t pt-4" onSubmit={(event) => void submit(event)}>
              <Textarea
                className="min-h-20 flex-1"
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
                placeholder="输入问题…"
              />
              <Button type="submit" disabled={asking || loadingHistory || !prompt.trim()}>
                <Send size={16} />
                发送
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
