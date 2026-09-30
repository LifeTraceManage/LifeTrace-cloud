import { type FormEvent } from "react";
import {
  Bot, Check, ChevronDown, History, MessageSquareText, Plus, Send, Trash2, X,
} from "lucide-react";
import { Badge, Button, Textarea, cn } from "../../components/ui";
import type { AssistantApproval } from "../../services/core";
import { useAgentSidebar } from "./AgentSidebarContext";

function approvalLabel(approval: AssistantApproval): string {
  const labels: Record<string, string> = {
    create_task: "创建任务",
    update_task: "修改任务",
    create_calendar_event: "创建日程",
    create_project: "创建 Project",
    update_project: "修改 Project",
    create_habit: "创建习惯",
    update_habit: "修改习惯",
    create_memo: "创建 Memo",
    update_memo: "修改 Memo",
    create_waiting_item: "创建 Waiting Item",
    update_waiting_item: "修改 Waiting Item",
    create_reminder: "创建提醒",
    update_reminder: "修改提醒",
    send_mail: "发送邮件",
    reply_mail: "回复邮件",
  };
  return labels[approval.actionName] ?? approval.actionName;
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
    const schedule = typeof action.scheduledStartAt === "string"
      ? ` · 安排 ${new Date(action.scheduledStartAt).toLocaleString()}`
      : "";
    const due = typeof action.dueAt === "string"
      ? ` · 截止 ${new Date(action.dueAt).toLocaleString()}`
      : "";
    return `${title || "未命名任务"}${schedule}${due}`;
  }
  if (approval.actionName === "update_task") {
    const parts = [
      typeof action.title === "string" ? `标题 → ${action.title}` : "",
      typeof action.status === "string" ? `状态 → ${action.status}` : "",
      typeof action.priority === "string" ? `优先级 → ${action.priority}` : "",
      typeof action.dueAt === "string" ? `截止 → ${new Date(action.dueAt).toLocaleString()}` : "",
      typeof action.scheduledStartAt === "string"
        ? `安排 → ${new Date(action.scheduledStartAt).toLocaleString()}`
        : "",
      typeof action.scheduledEndAt === "string"
        ? `结束 → ${new Date(action.scheduledEndAt).toLocaleString()}`
        : "",
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : `任务 ${String(action.taskId ?? "")}`;
  }
  if (approval.actionName === "create_calendar_event") {
    const when = typeof action.startAt === "string"
      ? new Date(action.startAt).toLocaleString()
      : typeof action.startLocalDate === "string"
        ? action.startLocalDate
        : "";
    return `${title || "未命名日程"}${when ? ` · ${when}` : ""}`;
  }
  if (approval.actionName === "create_project") return String(action.name ?? "未命名 Project");
  if (approval.actionName === "update_project") return String(action.name ?? action.projectId ?? "Project");
  if (approval.actionName === "create_habit") return String(action.name ?? "未命名习惯");
  if (approval.actionName === "update_habit") return String(action.name ?? action.habitId ?? "习惯");
  if (approval.actionName === "create_memo") {
    const value = String(action.content ?? "空 Memo");
    return value.length > 100 ? `${value.slice(0, 100)}…` : value;
  }
  if (approval.actionName === "update_memo") return `Memo ${String(action.memoId ?? "")}`;
  if (approval.actionName === "create_waiting_item") return String(action.title ?? "未命名等待事项");
  if (approval.actionName === "update_waiting_item") return String(action.title ?? action.waitingItemId ?? "Waiting Item");
  if (approval.actionName === "create_reminder") {
    const trigger = typeof action.triggerAt === "string" ? new Date(action.triggerAt).toLocaleString() : "";
    return `${String(action.subjectType ?? "对象")} ${String(action.subjectId ?? "")}${trigger ? ` · ${trigger}` : ""}`;
  }
  if (approval.actionName === "update_reminder") return `Reminder ${String(action.reminderId ?? "")}`;
  return "待确认写操作";
}

function mailApprovalDetail(approval: AssistantApproval): {
  to: string;
  cc: string;
  bcc: string;
  subject: string;
  body: string;
} | null {
  if (approval.actionName !== "send_mail" && approval.actionName !== "reply_mail") return null;
  const action = approval.actionJson;
  return {
    to: Array.isArray(action.to) ? action.to.map(String).join(", ") : "",
    cc: Array.isArray(action.cc) ? action.cc.map(String).join(", ") : "",
    bcc: Array.isArray(action.bcc) ? action.bcc.map(String).join(", ") : "",
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

export function AgentSidebar() {
  const agent = useAgentSidebar();
  if (!agent.open) return null;

  const pending = agent.approvals.filter((item) => item.status === "pending");
  const history = agent.approvals.filter((item) => item.status !== "pending").slice(0, 10);

  function submit(event: FormEvent) {
    event.preventDefault();
    void agent.send();
  }

  return (
    <>
      <button
        className="fixed inset-0 z-50 bg-black/35 lg:hidden"
        aria-label="关闭 Agent"
        onClick={() => agent.setOpen(false)}
      />
      <aside
        className="fixed inset-y-0 right-0 z-[60] flex w-full flex-col border-l bg-background shadow-2xl sm:max-w-md lg:z-40 lg:w-[420px] lg:max-w-none lg:shadow-none"
        aria-label="LifeTrace Agent"
      >
        <div className="flex h-14 shrink-0 items-center justify-between border-b px-4 lg:h-16">
          <div className="min-w-0">
            <div className="flex items-center gap-2 font-semibold"><Bot size={17} />Agent</div>
            <div className="mt-0.5 truncate text-[11px] text-muted-foreground">
              {agent.pageContext?.label ?? "LifeTrace"}
            </div>
          </div>
          <div className="flex items-center gap-1">
            <Button
              size="icon"
              variant={agent.historyOpen ? "secondary" : "ghost"}
              onClick={() => agent.setHistoryOpen(!agent.historyOpen)}
              aria-label="会话历史"
            >
              <History size={16} />
            </Button>
            <Button size="icon" variant="ghost" onClick={agent.newConversation} aria-label="新对话">
              <Plus size={16} />
            </Button>
            <Button size="icon" variant="ghost" onClick={() => agent.setOpen(false)} aria-label="关闭 Agent">
              <X size={17} />
            </Button>
          </div>
        </div>

        {agent.historyOpen ? (
          <div className="min-h-0 flex-1 overflow-y-auto p-3">
            <div className="mb-2 px-1 text-xs font-medium text-muted-foreground">会话历史</div>
            <div className="space-y-1">
              {agent.sessions.map((item) => (
                <div
                  key={item.id}
                  className={cn(
                    "group flex items-center rounded-md hover:bg-muted",
                    item.id === agent.sessionId && "bg-muted",
                  )}
                >
                  <button
                    className="min-w-0 flex-1 px-3 py-2 text-left"
                    onClick={() => void agent.openSession(item.id)}
                    disabled={agent.loadingHistory || agent.asking}
                  >
                    <div className="truncate text-sm font-medium">{item.title}</div>
                    <div className="mt-1 text-[11px] text-muted-foreground">
                      {new Date(item.lastMessageAt ?? item.updatedAt).toLocaleString()}
                    </div>
                  </button>
                  <button
                    className="mr-1 rounded p-2 text-muted-foreground opacity-0 hover:bg-background hover:text-destructive focus:opacity-100 group-hover:opacity-100"
                    onClick={() => void agent.deleteConversation(item.id)}
                    disabled={Boolean(agent.deletingSessionId)}
                    aria-label={`删除会话 ${item.title}`}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
              {!agent.sessions.length ? (
                <div className="px-3 py-8 text-center text-xs text-muted-foreground">暂无历史会话</div>
              ) : null}
            </div>
          </div>
        ) : (
          <>
            <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-4 py-4">
              {!agent.messages.length && !agent.loadingHistory ? (
                <div className="flex min-h-48 flex-col items-center justify-center text-center text-sm text-muted-foreground">
                  <MessageSquareText size={24} className="mb-3" />
                  <div className="font-medium text-foreground">直接问当前页面</div>
                  <div className="mt-1 max-w-64 text-xs leading-5">
                    Agent 会知道你当前所在的工作区；业务事实仍会通过云端工具重新核验。
                  </div>
                </div>
              ) : null}

              {agent.loadingHistory ? (
                <div className="text-sm text-muted-foreground">正在加载会话…</div>
              ) : null}

              {agent.messages.map((message, index) => (
                <div key={index} className={cn("flex", message.role === "user" ? "justify-end" : "justify-start")}>
                  <div
                    className={cn(
                      "max-w-[88%] rounded-lg px-3.5 py-2.5 text-sm leading-6",
                      message.role === "user" ? "bg-primary text-primary-foreground" : "bg-muted",
                    )}
                  >
                    <div className="whitespace-pre-wrap">{message.content}</div>
                    {message.provider ? <Badge className="mt-2">{message.provider}</Badge> : null}
                  </div>
                </div>
              ))}

              {pending.length ? (
                <div className="space-y-2 border-t pt-3">
                  <div className="text-xs font-medium text-muted-foreground">待确认操作</div>
                  {pending.map((approval) => (
                    <div key={approval.id} className="rounded-lg border bg-card p-3">
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <div className="text-sm font-medium">{approvalLabel(approval)}</div>
                          <div className="mt-1 text-xs leading-5 text-muted-foreground">{approvalSummary(approval)}</div>
                        </div>
                        <Badge>{statusLabel(approval)}</Badge>
                      </div>
                      {mailApprovalDetail(approval) ? (() => {
                        const detail = mailApprovalDetail(approval)!;
                        return <div className="mt-3 rounded-md border bg-muted/30 p-3 text-xs">
                          <div><span className="font-medium">To:</span> {detail.to}</div>
                          {detail.cc ? <div className="mt-1"><span className="font-medium">Cc:</span> {detail.cc}</div> : null}
                          {detail.bcc ? <div className="mt-1"><span className="font-medium">Bcc:</span> {detail.bcc}</div> : null}
                          <div className="mt-1"><span className="font-medium">Subject:</span> {detail.subject || "(无主题)"}</div>
                          <pre className="mt-3 max-h-48 overflow-y-auto whitespace-pre-wrap break-words border-t pt-3 font-sans leading-5">{detail.body}</pre>
                        </div>;
                      })() : null}
                      <div className="mt-3 flex gap-2">
                        <Button
                          size="sm"
                          onClick={() => void agent.decideApproval(approval.id, "approve")}
                          disabled={Boolean(agent.decidingApprovalId)}
                        >
                          <Check size={14} />{approval.actionName === "send_mail" || approval.actionName === "reply_mail" ? "批准并发送" : "批准并执行"}
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => void agent.decideApproval(approval.id, "reject")}
                          disabled={Boolean(agent.decidingApprovalId)}
                        >
                          <X size={14} />拒绝
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              ) : null}

              {history.length ? (
                <details className="border-t pt-3">
                  <summary className="flex cursor-pointer list-none items-center gap-2 text-xs text-muted-foreground">
                    <ChevronDown size={14} />最近审批历史 ({history.length})
                  </summary>
                  <div className="mt-2 space-y-2">
                    {history.map((approval) => (
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
                </details>
              ) : null}

              {agent.asking ? <div className="text-xs text-muted-foreground">正在查询并分析云端记录…</div> : null}
              {agent.error ? <div className="text-xs text-destructive">{agent.error}</div> : null}
            </div>

            <form className="shrink-0 border-t p-3" onSubmit={submit}>
              <Textarea
                className="min-h-20 resize-none"
                value={agent.draft}
                onChange={(event) => agent.setDraft(event.target.value)}
                placeholder="直接问当前页面，或让 Agent 执行操作…"
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault();
                    if (agent.draft.trim() && !agent.asking) void agent.send();
                  }
                }}
              />
              <div className="mt-2 flex items-center justify-between gap-2">
                <div className="min-w-0 truncate text-[11px] text-muted-foreground">
                  {agent.pageContext?.selectedEntity
                    ? `已选中 ${agent.pageContext.selectedEntity.entityType}`
                    : agent.pageContext?.label ?? "LifeTrace"}
                </div>
                <Button type="submit" size="sm" disabled={agent.asking || !agent.draft.trim()}>
                  <Send size={14} />发送
                </Button>
              </div>
            </form>
          </>
        )}
      </aside>
    </>
  );
}
