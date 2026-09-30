import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Bot, Check, ChevronDown, History, Plus, Send, Trash2, X } from "lucide-react";
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
  if (approval.actionName === "create_memo") return "创建 Memo";
  if (approval.actionName === "update_memo") return "修改 Memo";
  if (approval.actionName === "create_waiting_item") return "创建 Waiting Item";
  if (approval.actionName === "update_waiting_item") return "修改 Waiting Item";
  if (approval.actionName === "create_reminder") return "创建提醒";
  if (approval.actionName === "update_reminder") return "修改提醒";
  if (approval.actionName === "send_mail") return "发送邮件";
  if (approval.actionName === "reply_mail") return "回复邮件";
  return approval.actionName;
}

function approvalSummary(approval: AssistantApproval): string {
  const action = approval.actionJson;
  const title = typeof action.title === "string" ? action.title : "";
  if (approval.actionName === "send_mail" || approval.actionName === "reply_mail") {
    const to = Array.isArray(action.to) ? action.to.map(String).join("、") : "";
    const subject = typeof action.subject === "string" ? action.subject : "(无主题)";
    return `${to ? `To: ${to} · ` : ""}${subject}`;
  }
  if (approval.actionName === "create_task") {
    const schedule =
      typeof action.scheduledStartAt === "string"
        ? ` · 安排 ${new Date(action.scheduledStartAt).toLocaleString()}`
        : "";
    const due = typeof action.dueAt === "string" ? ` · 截止 ${new Date(action.dueAt).toLocaleString()}` : "";
    return `${title || "未命名任务"}${schedule}${due}`;
  }
  if (approval.actionName === "update_task") {
    const parts = [
      typeof action.title === "string" ? `标题 → ${action.title}` : "",
      typeof action.status === "string" ? `状态 → ${action.status}` : "",
      typeof action.priority === "string" ? `优先级 → ${action.priority}` : "",
      typeof action.dueAt === "string" ? `截止 → ${new Date(action.dueAt).toLocaleString()}` : "",
      action.clearDueAt === true ? "清除截止时间" : "",
      typeof action.scheduledStartAt === "string"
        ? `安排 → ${new Date(action.scheduledStartAt).toLocaleString()}`
        : "",
      typeof action.scheduledEndAt === "string"
        ? `结束 → ${new Date(action.scheduledEndAt).toLocaleString()}`
        : "",
      action.clearSchedule === true ? "移出 Planner 时间轴" : "",
      typeof action.estimatedMinutes === "number" ? `预计 ${action.estimatedMinutes} 分钟` : "",
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
  if (approval.actionName === "create_memo") {
    const content = typeof action.content === "string" ? action.content : "空 Memo";
    return content.length > 100 ? `${content.slice(0, 100)}…` : content;
  }
  if (approval.actionName === "update_memo") {
    const parts = [
      typeof action.content === "string" ? "修改内容" : "",
      action.isPinned === true ? "置顶" : action.isPinned === false ? "取消置顶" : "",
      typeof action.status === "string" ? `状态 → ${action.status}` : "",
      action.clearContext === true ? "清除上下文" : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : `Memo ${String(action.memoId ?? "")}`;
  }
  if (approval.actionName === "create_waiting_item") {
    const titleValue = typeof action.title === "string" ? action.title : "未命名等待事项";
    const waitingFor = typeof action.waitingFor === "string" ? ` · 等待 ${action.waitingFor}` : "";
    return `${titleValue}${waitingFor}`;
  }
  if (approval.actionName === "update_waiting_item") {
    const parts = [
      typeof action.title === "string" ? `标题 → ${action.title}` : "",
      typeof action.waitingFor === "string" ? `等待 → ${action.waitingFor}` : "",
      typeof action.status === "string" ? `状态 → ${action.status}` : "",
      typeof action.followUpAt === "string" ? `跟进 → ${new Date(action.followUpAt).toLocaleString()}` : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : `Waiting Item ${String(action.waitingItemId ?? "")}`;
  }
  if (approval.actionName === "create_reminder") {
    const subject = `${String(action.subjectType ?? "对象")} ${String(action.subjectId ?? "")}`;
    const trigger = typeof action.triggerAt === "string" ? new Date(action.triggerAt).toLocaleString() : "";
    return `${subject}${trigger ? ` · ${trigger}` : ""}`;
  }
  if (approval.actionName === "update_reminder") {
    const parts = [
      typeof action.triggerAt === "string" ? `提醒时间 → ${new Date(action.triggerAt).toLocaleString()}` : "",
      typeof action.snoozedUntil === "string" ? `稍后提醒 → ${new Date(action.snoozedUntil).toLocaleString()}` : "",
      typeof action.status === "string" ? `状态 → ${action.status}` : "",
      action.clearSnooze === true ? "清除稍后提醒" : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : `Reminder ${String(action.reminderId ?? "")}`;
  }
  return "待确认写操作";
}

function mailApprovalDetail(approval: AssistantApproval): {
  to: string;
  cc: string;
  subject: string;
  body: string;
} | null {
  if (approval.actionName !== "send_mail" && approval.actionName !== "reply_mail") return null;
  const action = approval.actionJson;
  return {
    to: Array.isArray(action.to) ? action.to.map(String).join(", ") : "",
    cc: Array.isArray(action.cc) ? action.cc.map(String).join(", ") : "",
    subject: typeof action.subject === "string" ? action.subject : "",
    body: typeof action.bodyText === "string" ? action.bodyText : "",
  };
}

function statusLabel(approval: AssistantApproval): string {
  if (approval.status === "pending") return "待确认";
  if (approval.status === "approved") return "已执行";
  if (approval.status === "rejected") return "已拒绝";
  if (approval.status === "expired") return "已过期";
  if (approval.status === "cancelled" && approval.cancellationReason === "superseded") return "已被新方案替代";
  if (approval.status === "cancelled") return "已取消";
  return approval.status;
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
  const [deletingSessionId, setDeletingSessionId] = useState<string | null>(null);
  const [showApprovalHistory, setShowApprovalHistory] = useState(false);
  const [error, setError] = useState("");
  const pendingApprovals = approvals.filter((item) => item.status === "pending");
  const approvalHistory = approvals.filter((item) => item.status !== "pending").slice(0, 10);

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
      setShowApprovalHistory(false);
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
    setShowApprovalHistory(false);
    setPrompt("");
    setError("");
  }

  async function deleteConversation(id: string) {
    if (!session || deletingSessionId || asking) return;
    const target = sessions.find((item) => item.id === id);
    if (!window.confirm(`删除会话“${target?.title ?? "未命名会话"}”？该会话的消息和审批记录也会一起删除。`)) return;
    setDeletingSessionId(id);
    setError("");
    try {
      await api.deleteSession(id, session.csrfToken);
      setSessions((items) => items.filter((item) => item.id !== id));
      if (sessionId === id) {
        newConversation();
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "会话删除失败");
    } finally {
      setDeletingSessionId(null);
    }
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
      <div className="mx-auto grid h-[calc(100vh-9rem)] min-h-[600px] max-w-6xl gap-4 lg:grid-cols-[260px_minmax(0,1fr)]">
        <Card className="min-h-0 overflow-hidden">
          <CardContent className="flex h-full min-h-0 flex-col gap-3 pt-5">
            <Button className="w-full justify-start" variant="outline" onClick={newConversation} disabled={asking}>
              <Plus size={16} />
              新对话
            </Button>
            <div className="min-h-0 flex-1 space-y-1 overflow-y-auto pr-1">
              {sessions.map((item) => (
                <div
                  key={item.id}
                  className={`group flex items-center gap-1 rounded-md transition-colors hover:bg-muted ${
                    item.id === sessionId ? "bg-muted font-medium" : ""
                  }`}
                >
                  <button
                    type="button"
                    onClick={() => void openSession(item.id)}
                    disabled={asking || loadingHistory}
                    className="min-w-0 flex-1 px-3 py-2 text-left text-sm"
                  >
                    <div className="truncate">{item.title}</div>
                    <div className="mt-1 text-xs text-muted-foreground">
                      {new Date(item.lastMessageAt ?? item.updatedAt).toLocaleString()}
                    </div>
                  </button>
                  <button
                    type="button"
                    title="删除会话"
                    aria-label={`删除会话 ${item.title}`}
                    onClick={() => void deleteConversation(item.id)}
                    disabled={Boolean(deletingSessionId) || asking}
                    className="mr-1 rounded p-2 text-muted-foreground opacity-0 hover:bg-background hover:text-destructive focus:opacity-100 group-hover:opacity-100"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
              {!sessions.length ? (
                <div className="px-2 py-3 text-xs text-muted-foreground">暂无历史会话</div>
              ) : null}
            </div>
          </CardContent>
        </Card>

        <Card className="min-h-0 overflow-hidden">
          <CardContent className="flex h-full min-h-0 flex-col pt-5">
            <div className="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1">
              {loadingHistory ? (
                <div className="text-sm text-muted-foreground">正在加载会话…</div>
              ) : !messages.length ? (
                <EmptyState
                  icon={<Bot size={26} />}
                  title="问问 LifeTrace"
                  description="可以查询已同步数据并主动规划任务时间；所有写操作都会先生成审批，只有你确认后才执行。"
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
              {pendingApprovals.length ? (
                <div className="space-y-3 border-t pt-4">
                  <div className="text-xs font-medium text-muted-foreground">待确认操作</div>
                  {pendingApprovals.map((approval) => (
                    <div key={approval.id} className="rounded-lg border bg-card px-4 py-3">
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <div>
                          <div className="text-sm font-medium">{approvalLabel(approval)}</div>
                          <div className="mt-1 text-xs text-muted-foreground">{approvalSummary(approval)}</div>
                        </div>
                        <Badge>{statusLabel(approval)}</Badge>
                      </div>
                      {mailApprovalDetail(approval) ? (() => {
                        const detail = mailApprovalDetail(approval)!;
                        return <div className="mt-3 rounded-md border bg-muted/30 p-3 text-xs">
                          <div><span className="font-medium">To:</span> {detail.to}</div>
                          {detail.cc ? <div className="mt-1"><span className="font-medium">Cc:</span> {detail.cc}</div> : null}
                          <div className="mt-1"><span className="font-medium">Subject:</span> {detail.subject || "(无主题)"}</div>
                          <pre className="mt-3 max-h-48 overflow-y-auto whitespace-pre-wrap break-words border-t pt-3 font-sans leading-5">{detail.body}</pre>
                        </div>;
                      })() : null}
                      <div className="mt-3 flex gap-2">
                        <Button
                          size="sm"
                          onClick={() => void decideApproval(approval.id, "approve")}
                          disabled={Boolean(decidingApprovalId)}
                        >
                          <Check size={14} />
                          {approval.actionName === "send_mail" || approval.actionName === "reply_mail" ? "批准并发送" : "批准并执行"}
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
                    </div>
                  ))}
                </div>
              ) : null}
              {approvalHistory.length ? (
                <div className="border-t pt-3">
                  <button
                    type="button"
                    onClick={() => setShowApprovalHistory((value) => !value)}
                    className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground"
                  >
                    <History size={14} />
                    最近审批历史 ({approvalHistory.length})
                    <ChevronDown
                      size={14}
                      className={showApprovalHistory ? "rotate-180 transition-transform" : "transition-transform"}
                    />
                  </button>
                  {showApprovalHistory ? (
                    <div className="mt-3 space-y-2">
                      {approvalHistory.map((approval) => (
                        <div key={approval.id} className="rounded-md border bg-muted/30 px-3 py-2">
                          <div className="flex items-center justify-between gap-2">
                            <div className="min-w-0">
                              <div className="truncate text-xs font-medium">{approvalLabel(approval)}</div>
                              <div className="mt-1 truncate text-xs text-muted-foreground">{approvalSummary(approval)}</div>
                            </div>
                            <Badge>{statusLabel(approval)}</Badge>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : null}
                </div>
              ) : null}
              {asking ? <div className="text-sm text-muted-foreground">正在查询并分析云端记录…</div> : null}
              {error ? <div className="text-sm text-destructive">{error}</div> : null}
            </div>
            <form className="mt-4 flex shrink-0 items-end gap-2 border-t pt-4" onSubmit={(event) => void submit(event)}>
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
