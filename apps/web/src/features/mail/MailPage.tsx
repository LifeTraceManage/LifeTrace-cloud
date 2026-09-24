import { useDeferredValue, useMemo, useState } from "react";
import {
  Archive, ArrowLeft, CalendarPlus, CheckSquare2, Clock3, FileText, Inbox, Loader2,
  Mail, MailOpen, NotebookPen, PenLine, RefreshCw, Reply, ReplyAll, RotateCcw,
  Search, Send, Settings, Star, Trash2,
} from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Dialog, EmptyState, Input, cn } from "../../components/ui";
import {
  createEntityLink,
  createExecutionCalendarEvent,
  createExecutionTask,
  createExecutionWaitingItem,
  createNote,
} from "../../services/core";
import { WorkspaceShell } from "../../layouts/WorkspaceShell";
import { MailAccountSettings } from "./MailAccountSettings";
import { MailComposer, type ComposeMode } from "./MailComposer";
import { renderableMailHtml } from "./mailHtml";
import type { MailAddress, MailDraft, MailMessageDetail } from "./types";
import { useMailWorkspace } from "./useMailWorkspace";

const mailboxes = [
  { id: "inbox", label: "收件箱", icon: Inbox },
  { id: "starred", label: "星标", icon: Star },
  { id: "sent", label: "已发送", icon: Send },
  { id: "drafts", label: "草稿", icon: FileText },
  { id: "archive", label: "归档", icon: Archive },
  { id: "trash", label: "废纸篓", icon: Trash2 },
] as const;

type MailboxId = typeof mailboxes[number]["id"];
type IntegrationMode = "calendar" | "waiting" | null;

function formatAddress(address: MailAddress): string {
  return address.name ? `${address.name} <${address.email}>` : address.email;
}

function addressesText(addresses?: MailAddress[]): string {
  return addresses?.map(formatAddress).join(", ") ?? "";
}

function plainBody(message: MailMessageDetail | null): string {
  if (!message) return "";
  if (message.text) return message.text;
  if (!message.html) return "";
  if (typeof DOMParser === "undefined") return "该邮件仅包含 HTML 正文。";
  const document = new DOMParser().parseFromString(message.html, "text/html");
  return document.body.textContent?.trim() || "";
}

function safeAttachmentUrl(value?: string | null): string | null {
  if (!value || typeof window === "undefined") return null;
  try {
    const url = new URL(value, window.location.origin);
    return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : null;
  } catch {
    return null;
  }
}

function mailSource(message: MailMessageDetail): string {
  return `mail://message/${message.id}`;
}

function mailContextMarkdown(message: MailMessageDetail): string {
  const body = plainBody(message);
  return [
    `# ${message.subject || "邮件"}`,
    "",
    `- From: ${addressesText(message.from) || "未知"}`,
    `- To: ${addressesText(message.to) || "未知"}`,
    `- Date: ${message.sentAt}`,
    `- Source: ${mailSource(message)}`,
    "",
    "---",
    "",
    body,
  ].join("\n");
}

function inputDateTime(value: string): string | null {
  if (!value.trim()) return null;
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString();
}

export function MailPage() {
  const { session, upsert } = useApp();
  const [mailbox, setMailbox] = useState<MailboxId>("inbox");
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [accountId, setAccountId] = useState<string | null>(null);
  const [mobileDetail, setMobileDetail] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [composeOpen, setComposeOpen] = useState(false);
  const [composeMode, setComposeMode] = useState<ComposeMode>("new");
  const [composeSource, setComposeSource] = useState<MailMessageDetail | null>(null);
  const [composeDraft, setComposeDraft] = useState<MailDraft | null>(null);
  const [notice, setNotice] = useState("");
  const [integrationMode, setIntegrationMode] = useState<IntegrationMode>(null);
  const [integrationTitle, setIntegrationTitle] = useState("");
  const [calendarStart, setCalendarStart] = useState("");
  const [calendarEnd, setCalendarEnd] = useState("");
  const [waitingFor, setWaitingFor] = useState("");
  const [followUpAt, setFollowUpAt] = useState("");
  const [integrationBusy, setIntegrationBusy] = useState(false);

  const {
    runtime, accounts, identities, drafts, messages, selectedId, selectedMessage,
    runtimeLoading, listLoading, detailLoading, error,
    setSelectedId, refresh, send, markRead, setStarred, move,
    connectAccount, disconnectAccount, testAccount, syncAccount,
    createIdentity, updateIdentity, deleteIdentity,
    saveDraft, deleteDraft, sendDraft,
    listDraftAttachments, uploadDraftAttachment, deleteDraftAttachment,
  } = useMailWorkspace(mailbox, deferredQuery, accountId);

  const current = mailboxes.find((item) => item.id === mailbox) ?? mailboxes[0];
  const runtimeReady = runtime.status === "ready";
  const account = accounts.find((item) => item.id === accountId) ?? null;
  const messageBody = useMemo(() => plainBody(selectedMessage), [selectedMessage]);
  const renderedHtml = useMemo(() => selectedMessage ? renderableMailHtml(selectedMessage) : "", [selectedMessage]);
  const unreadCount = messages.filter((message) => !message.isRead).length;
  const visibleDrafts = useMemo(() => {
    const needle = deferredQuery.trim().toLocaleLowerCase("zh-CN");
    return drafts.filter((draft) => {
      if (accountId && draft.accountId !== accountId) return false;
      if (!needle) return true;
      return `${draft.subject} ${draft.bodyText} ${addressesText(draft.to)}`
        .toLocaleLowerCase("zh-CN")
        .includes(needle);
    });
  }, [accountId, deferredQuery, drafts]);
  const listCount = mailbox === "drafts" ? visibleDrafts.length : messages.length;

  function changeMailbox(next: MailboxId) {
    setMailbox(next);
    setMobileDetail(false);
    setSelectedId(null);
  }

  function selectMessage(id: string) {
    setSelectedId(id);
    setMobileDetail(true);
  }

  function openComposer(
    mode: ComposeMode,
    source: MailMessageDetail | null = null,
    draft: MailDraft | null = null,
  ) {
    setComposeMode(mode);
    setComposeSource(source);
    setComposeDraft(draft);
    setComposeOpen(true);
  }

  function openIntegration(mode: Exclude<IntegrationMode, null>) {
    if (!selectedMessage) return;
    setIntegrationMode(mode);
    setIntegrationTitle(selectedMessage.subject || "邮件事项");
    setWaitingFor(addressesText(selectedMessage.from));
    setCalendarStart("");
    setCalendarEnd("");
    setFollowUpAt("");
  }

  async function saveMailToNotes() {
    if (!session || !selectedMessage) return;
    setIntegrationBusy(true);
    try {
      const note = createNote(
        session.user.id,
        session.session.deviceId,
        selectedMessage.subject || "邮件",
        mailContextMarkdown(selectedMessage),
      );
      await upsert("note.note", note);
      await upsert("entity.link", createEntityLink(
        session.user.id,
        session.session.deviceId,
        "mail.message",
        selectedMessage.id,
        "created_from",
        "note.note",
        note.meta.id,
        { sourceUri: mailSource(selectedMessage), accountId: selectedMessage.accountId },
      ));
      setNotice("邮件已保存到 Notes，并保留来源链接。");
    } finally {
      setIntegrationBusy(false);
    }
  }

  async function createTaskFromMail() {
    if (!session || !selectedMessage) return;
    setIntegrationBusy(true);
    try {
      const task = createExecutionTask(session.user.id, session.session.deviceId, {
        title: selectedMessage.subject || "处理邮件",
        description: `${plainBody(selectedMessage).slice(0, 2000)}\n\nSource: ${mailSource(selectedMessage)}`,
        context: "inbox",
      });
      await upsert("execution.task", task);
      await upsert("entity.link", createEntityLink(
        session.user.id,
        session.session.deviceId,
        "mail.message",
        selectedMessage.id,
        "created_from",
        "execution.task",
        task.meta.id,
        { sourceUri: mailSource(selectedMessage) },
      ));
      setNotice("已从邮件创建 Inbox Task。");
    } finally {
      setIntegrationBusy(false);
    }
  }

  async function submitIntegration() {
    if (!session || !selectedMessage || !integrationMode) return;
    setIntegrationBusy(true);
    try {
      if (integrationMode === "calendar") {
        const startAt = inputDateTime(calendarStart);
        if (!startAt) throw new Error("请选择有效开始时间");
        const endAt = inputDateTime(calendarEnd);
        const event = createExecutionCalendarEvent(session.user.id, session.session.deviceId, {
          title: integrationTitle,
          description: `${plainBody(selectedMessage).slice(0, 2000)}\n\nSource: ${mailSource(selectedMessage)}`,
          startAt,
          endAt,
        });
        await upsert("execution.calendar_event", event);
        await upsert("entity.link", createEntityLink(
          session.user.id,
          session.session.deviceId,
          "mail.message",
          selectedMessage.id,
          "created_from",
          "execution.calendar_event",
          event.meta.id,
          { sourceUri: mailSource(selectedMessage) },
        ));
        setNotice("已从邮件创建日历事件。");
      } else {
        const item = createExecutionWaitingItem(session.user.id, session.session.deviceId, {
          title: integrationTitle,
          description: `${plainBody(selectedMessage).slice(0, 2000)}\n\nSource: ${mailSource(selectedMessage)}`,
          waitingFor: waitingFor.trim() || addressesText(selectedMessage.from) || "邮件联系人",
          followUpAt: inputDateTime(followUpAt),
        });
        await upsert("execution.waiting_item", item);
        await upsert("entity.link", createEntityLink(
          session.user.id,
          session.session.deviceId,
          "mail.message",
          selectedMessage.id,
          "created_from",
          "execution.waiting_item",
          item.meta.id,
          { sourceUri: mailSource(selectedMessage) },
        ));
        setNotice("已从邮件创建 Waiting Item。");
      }
      setIntegrationMode(null);
    } catch (cause) {
      setNotice(cause instanceof Error ? cause.message : "创建失败");
    } finally {
      setIntegrationBusy(false);
    }
  }

  const runtimeDescription = runtimeReady
    ? accounts.length
      ? "LifeTrace Cloud Mail 已连接，后台 Worker 会持续同步外部邮箱。"
      : "Mail 服务已就绪；连接第一个外部邮箱后即可开始同步。"
    : "LifeTrace Cloud Mail 暂不可用。请确认 Cloud 数据库、MAIL_CREDENTIAL_KEY 和当前会话 mail scopes。";

  return <WorkspaceShell
    title="Mail"
    icon={<Mail size={17} />}
    action={<>
      <Button size="sm" variant="outline" onClick={() => setSettingsOpen(true)}><Settings size={14} /><span className="hidden sm:inline">设置</span></Button>
      <Button className="hidden sm:inline-flex" size="sm" variant="outline" onClick={() => void refresh()} disabled={runtimeLoading}><RefreshCw size={14} />刷新</Button>
      <Button size="sm" disabled={!runtimeReady || !identities.length} onClick={() => openComposer("new")}><PenLine size={15} /><span className="hidden sm:inline">写邮件</span></Button>
    </>}
  >
    {notice ? <div className="mx-3 mt-3 flex items-center justify-between gap-3 rounded-md border bg-card px-3 py-2 text-xs sm:mx-5 lg:mx-6"><span>{notice}</span><button className="text-muted-foreground" onClick={() => setNotice("")}>关闭</button></div> : null}

    <div className="grid min-h-[calc(100vh-6rem)] lg:min-h-[calc(100vh-4rem)] lg:grid-cols-[228px_360px_minmax(0,1fr)]">
      <aside className="border-b bg-card/45 p-3 lg:border-b-0 lg:border-r">
        <div className="mb-4 flex items-center justify-between px-2 pt-1">
          <div>
            <div className="text-xs font-semibold">邮箱</div>
            <div className="mt-0.5 text-[11px] text-muted-foreground">{account?.email ?? "Unified Mailbox"}</div>
          </div>
          <Badge>{accounts.length} 账号</Badge>
        </div>

        <nav className="grid grid-cols-3 gap-1 sm:grid-cols-6 lg:block lg:space-y-1" aria-label="邮箱文件夹">
          {mailboxes.map(({ id, label, icon: Icon }) => <button
            key={id}
            onClick={() => changeMailbox(id)}
            className={cn("flex h-9 items-center justify-center gap-2 rounded-md px-2.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground lg:w-full lg:justify-start lg:text-sm", mailbox === id && "bg-accent font-medium text-accent-foreground")}
          ><Icon size={16} /><span>{label}</span></button>)}
        </nav>

        <div className="mt-5 hidden border-t pt-4 lg:block">
          <div className="mb-2 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Accounts</div>
          <button onClick={() => setAccountId(null)} className={cn("mb-1 flex h-9 w-full items-center gap-2 rounded-md px-2.5 text-left text-xs text-muted-foreground hover:bg-muted", accountId === null && "bg-accent font-medium text-accent-foreground")}><Inbox size={14} /><span className="flex-1">所有邮箱</span></button>
          {accounts.map((item) => <button key={item.id} onClick={() => setAccountId(item.id)} className={cn("mb-1 flex min-h-9 w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs text-muted-foreground hover:bg-muted", accountId === item.id && "bg-accent font-medium text-accent-foreground")}>
            <span className={cn("h-2 w-2 shrink-0 rounded-full", item.status === "active" ? "bg-success" : item.status === "degraded" || item.status === "validating" ? "bg-warning" : "bg-muted-foreground")} />
            <span className="min-w-0 flex-1"><span className="block truncate">{item.displayName || item.email}</span><span className="block truncate text-[10px] opacity-75">{item.email}</span></span>
          </button>)}
          {!accounts.length ? <div className="rounded-md border border-dashed px-3 py-3 text-xs leading-5 text-muted-foreground">尚未连接邮箱账号。</div> : null}
          <Button className="mt-2 w-full justify-start" variant="ghost" size="sm" onClick={() => setSettingsOpen(true)}><Settings size={15} />账号设置</Button>
        </div>
      </aside>

      <section className={cn("min-w-0 border-b lg:block lg:border-b-0 lg:border-r", mobileDetail && "hidden lg:block")}>
        <div className="border-b p-3">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h1 className="text-sm font-semibold">{account ? `${account.email} · ${current.label}` : `所有邮箱 · ${current.label}`}</h1>
              <p className="mt-0.5 text-[11px] text-muted-foreground">
                {runtimeReady ? mailbox === "drafts" ? `${visibleDrafts.length} 个草稿` : `${messages.length} 封 · ${unreadCount} 封未读` : runtimeDescription}
              </p>
            </div>
            {listLoading ? <Loader2 size={15} className="animate-spin text-muted-foreground" /> : <Badge>{listCount}</Badge>}
          </div>
          <div className="relative mt-3">
            <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" size={15} />
            <Input className="h-9 pl-9" placeholder={mailbox === "drafts" ? "搜索草稿" : "搜索主题、发件人、收件人或正文"} value={query} onChange={(event) => setQuery(event.target.value)} disabled={!runtimeReady} />
          </div>
        </div>

        <div className="scrollbar-thin max-h-[calc(100vh-12rem)] overflow-y-auto p-2 lg:max-h-[calc(100vh-8rem)]">
          {error ? <div className="mb-2 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">{error}</div> : null}
          {runtimeLoading ? <div className="flex items-center justify-center gap-2 py-10 text-sm text-muted-foreground"><Loader2 size={16} className="animate-spin" />正在连接 LifeTrace Mail…</div> : null}
          {!runtimeLoading && !runtimeReady ? <EmptyState icon={<Inbox size={22} />} title="Mail 服务尚未就绪" description={runtimeDescription} action={<Button size="sm" variant="outline" onClick={() => setSettingsOpen(true)}><Settings size={14} />检查账号配置</Button>} /> : null}

          {runtimeReady && mailbox === "drafts" ? <>
            {visibleDrafts.map((draft) => <div key={draft.id} className="mb-1 rounded-md px-3 py-3 hover:bg-muted">
              <button className="w-full text-left" onClick={() => openComposer("draft", null, draft)}>
                <div className="flex items-center gap-2"><FileText size={13} className="text-muted-foreground" /><span className="min-w-0 flex-1 truncate text-sm font-medium">{draft.subject || "(无主题草稿)"}</span><span className="text-[10px] text-muted-foreground">{new Date(draft.updatedAt).toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}</span></div>
                <div className="mt-1 truncate text-xs text-muted-foreground">{addressesText(draft.to) || "尚未填写收件人"}</div>
                <div className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{draft.bodyText || "空草稿"}</div>
              </button>
              <div className="mt-2 flex justify-end"><Button size="sm" variant="ghost" onClick={() => void deleteDraft(draft.id)}><Trash2 size={13} />删除草稿</Button></div>
            </div>)}
            {!visibleDrafts.length ? <EmptyState icon={<FileText size={22} />} title="没有草稿" /> : null}
          </> : <>
            {runtimeReady && !listLoading && !messages.length ? <EmptyState icon={<MailOpen size={22} />} title={query ? "没有匹配的邮件" : `${current.label}为空`} /> : null}
            {messages.map((message) => <button key={message.id} onClick={() => selectMessage(message.id)} className={cn("mb-1 w-full rounded-md px-3 py-3 text-left transition-colors hover:bg-muted", selectedId === message.id && "bg-accent", !message.isRead && "font-medium")}>
              <div className="flex items-center gap-2">
                <span className="min-w-0 flex-1 truncate text-xs">{addressesText(message.from) || "未知发件人"}</span>
                {message.isStarred ? <Star size={12} className="shrink-0 fill-current text-warning" /> : null}
                <span className="shrink-0 text-[10px] font-normal text-muted-foreground">{new Date(message.sentAt).toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}</span>
              </div>
              <div className="mt-1 truncate text-sm">{message.subject || "(无主题)"}</div>
              <div className="mt-1 line-clamp-2 text-xs font-normal leading-5 text-muted-foreground">{message.preview || "无预览"}</div>
            </button>)}
          </>}
        </div>
      </section>

      <main className={cn("min-w-0 bg-background", mobileDetail ? "block" : "hidden lg:block")}>
        {selectedMessage && mailbox !== "drafts" ? <div className="flex min-h-full flex-col">
          <div className="border-b p-4 sm:p-5">
            <div className="mb-3 flex items-center gap-2">
              <Button className="lg:hidden" size="icon" variant="ghost" onClick={() => setMobileDetail(false)} aria-label="返回邮件列表"><ArrowLeft size={17} /></Button>
              <div className="min-w-0 flex-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em]">{selectedMessage.subject || "(无主题)"}</h2>
                <div className="mt-1 text-xs text-muted-foreground">{new Date(selectedMessage.sentAt).toLocaleString("zh-CN")}</div>
              </div>
              <Button size="icon" variant="ghost" aria-label={selectedMessage.isStarred ? "取消星标" : "添加星标"} onClick={() => void setStarred(selectedMessage.id, !selectedMessage.isStarred)}><Star size={16} className={selectedMessage.isStarred ? "fill-current text-warning" : ""} /></Button>
              <Button size="icon" variant="ghost" aria-label={selectedMessage.isRead ? "标记为未读" : "标记为已读"} onClick={() => void markRead(selectedMessage.id, !selectedMessage.isRead)}>{selectedMessage.isRead ? <Mail size={16} /> : <MailOpen size={16} />}</Button>
              {mailbox === "trash"
                ? <Button size="icon" variant="ghost" aria-label="恢复到收件箱" onClick={() => void move(selectedMessage.id, "inbox")}><RotateCcw size={16} /></Button>
                : <>
                  <Button size="icon" variant="ghost" aria-label="归档" onClick={() => void move(selectedMessage.id, "archive")}><Archive size={16} /></Button>
                  <Button size="icon" variant="ghost" aria-label="移到废纸篓" onClick={() => void move(selectedMessage.id, "trash")}><Trash2 size={16} /></Button>
                </>}
            </div>
            <div className="space-y-1 text-xs leading-5 text-muted-foreground">
              <div><span className="font-medium text-foreground">From:</span> {addressesText(selectedMessage.from)}</div>
              <div><span className="font-medium text-foreground">To:</span> {addressesText(selectedMessage.to)}</div>
              {selectedMessage.cc?.length ? <div><span className="font-medium text-foreground">Cc:</span> {addressesText(selectedMessage.cc)}</div> : null}
            </div>
            <div className="mt-4 flex flex-wrap gap-2">
              <Button size="sm" variant="outline" disabled={!identities.length} onClick={() => openComposer("reply", selectedMessage)}><Reply size={14} />回复</Button>
              <Button size="sm" variant="outline" disabled={!identities.length} onClick={() => openComposer("replyAll", selectedMessage)}><ReplyAll size={14} />回复全部</Button>
              <Button size="sm" variant="outline" disabled={!identities.length} onClick={() => openComposer("forward", selectedMessage)}><Send size={14} />转发</Button>
              <Button size="sm" variant="ghost" disabled={integrationBusy} onClick={() => void saveMailToNotes()}><NotebookPen size={14} />保存到 Notes</Button>
              <Button size="sm" variant="ghost" disabled={integrationBusy} onClick={() => void createTaskFromMail()}><CheckSquare2 size={14} />创建 Task</Button>
              <Button size="sm" variant="ghost" onClick={() => openIntegration("calendar")}><CalendarPlus size={14} />创建日程</Button>
              <Button size="sm" variant="ghost" onClick={() => openIntegration("waiting")}><Clock3 size={14} />等待回复</Button>
            </div>
          </div>

          <div className="scrollbar-thin flex-1 overflow-y-auto p-4 sm:p-6">
            {detailLoading ? <div className="flex items-center gap-2 text-sm text-muted-foreground"><Loader2 size={16} className="animate-spin" />加载邮件正文…</div> : selectedMessage.html ? <div
              className="break-words text-sm leading-7 text-foreground [&_a]:text-primary [&_a]:underline [&_blockquote]:border-l-2 [&_blockquote]:pl-3 [&_pre]:overflow-x-auto [&_pre]:whitespace-pre-wrap"
              dangerouslySetInnerHTML={{ __html: renderedHtml }}
            /> : <pre className="whitespace-pre-wrap break-words font-sans text-sm leading-7 text-foreground">{messageBody || "该邮件没有可显示的正文。"}</pre>}
            {selectedMessage.attachments.length ? <div className="mt-8 border-t pt-4">
              <div className="mb-3 text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">附件</div>
              <div className="grid gap-2 sm:grid-cols-2">{selectedMessage.attachments.map((attachment) => {
                const url = safeAttachmentUrl(attachment.downloadUrl);
                const item = <div className="rounded-md border bg-card px-3 py-2 text-xs"><div className="truncate font-medium">{attachment.filename}</div><div className="mt-1 text-muted-foreground">{attachment.contentType || "文件"}{attachment.sizeBytes ? ` · ${Math.ceil(attachment.sizeBytes / 1024)} KB` : ""}</div></div>;
                return url ? <a key={attachment.id} href={url} target="_blank" rel="noreferrer">{item}</a> : <div key={attachment.id}>{item}</div>;
              })}</div>
            </div> : null}
          </div>
        </div> : <div className="flex min-h-[calc(100vh-8rem)] items-center justify-center p-8">
          <div className="max-w-md text-center">
            <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">{detailLoading ? <Loader2 size={21} className="animate-spin" /> : <Mail size={21} />}</div>
            <h2 className="mt-4 text-lg font-semibold">{runtimeReady ? mailbox === "drafts" ? "选择或新建草稿" : "选择一封邮件" : "LifeTrace Mail"}</h2>
            {!runtimeReady ? <p className="mt-2 text-sm leading-6 text-muted-foreground">{runtimeDescription}</p> : null}
          </div>
        </div>}
      </main>
    </div>

    <MailAccountSettings
      open={settingsOpen}
      onOpenChange={setSettingsOpen}
      accounts={accounts}
      identities={identities}
      onConnect={connectAccount}
      onDisconnect={disconnectAccount}
      onTest={testAccount}
      onSync={syncAccount}
      onCreateIdentity={createIdentity}
      onUpdateIdentity={updateIdentity}
      onDeleteIdentity={deleteIdentity}
    />

    <MailComposer
      open={composeOpen}
      onOpenChange={setComposeOpen}
      mode={composeMode}
      sourceMessage={composeSource}
      draft={composeDraft}
      identities={identities}
      onSend={send}
      onSaveDraft={saveDraft}
      onSendDraft={sendDraft}
      onListAttachments={listDraftAttachments}
      onUploadAttachment={uploadDraftAttachment}
      onDeleteAttachment={deleteDraftAttachment}
    />

    <Dialog
      open={integrationMode !== null}
      onOpenChange={(open) => { if (!open) setIntegrationMode(null); }}
      title={integrationMode === "calendar" ? "从邮件创建日程" : "从邮件创建 Waiting Item"}
      description={selectedMessage ? `来源将保留为 ${mailSource(selectedMessage)}` : undefined}
    >
      <div className="space-y-3">
        <label className="block space-y-1.5 text-xs font-medium">标题<Input value={integrationTitle} onChange={(event) => setIntegrationTitle(event.target.value)} /></label>
        {integrationMode === "calendar" ? <>
          <label className="block space-y-1.5 text-xs font-medium">开始时间<Input type="datetime-local" value={calendarStart} onChange={(event) => setCalendarStart(event.target.value)} /></label>
          <label className="block space-y-1.5 text-xs font-medium">结束时间（可选）<Input type="datetime-local" value={calendarEnd} onChange={(event) => setCalendarEnd(event.target.value)} /></label>
        </> : <>
          <label className="block space-y-1.5 text-xs font-medium">等待对象<Input value={waitingFor} onChange={(event) => setWaitingFor(event.target.value)} /></label>
          <label className="block space-y-1.5 text-xs font-medium">跟进时间（可选）<Input type="datetime-local" value={followUpAt} onChange={(event) => setFollowUpAt(event.target.value)} /></label>
        </>}
        <div className="flex justify-end gap-2 border-t pt-3"><Button variant="ghost" onClick={() => setIntegrationMode(null)}>取消</Button><Button disabled={integrationBusy} onClick={() => void submitIntegration()}>{integrationBusy ? <Loader2 size={14} className="animate-spin" /> : null}创建</Button></div>
      </div>
    </Dialog>
  </WorkspaceShell>;
}
