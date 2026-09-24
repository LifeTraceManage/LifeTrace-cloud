import { useEffect, useMemo, useRef, useState } from "react";
import { FileText, Loader2, Paperclip, Save, Send, Trash2 } from "lucide-react";
import { Button, Dialog, Input, Select, Textarea } from "../../components/ui";
import type {
  ComposeMailInput,
  MailAddress,
  MailDraft,
  MailDraftAttachment,
  MailDraftInput,
  MailIdentity,
  MailMessageDetail,
} from "./types";

export type ComposeMode = "new" | "reply" | "replyAll" | "forward" | "draft";

const MAX_ATTACHMENT_BYTES = 18 * 1024 * 1024;

function formatAddress(address: MailAddress): string {
  return address.name ? `${address.name} <${address.email}>` : address.email;
}

function addressesText(addresses?: MailAddress[]): string {
  return addresses?.map(formatAddress).join(", ") ?? "";
}

function parseAddresses(value: string): MailAddress[] {
  return value
    .split(/[;,\n]/)
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const match = part.match(/^(?:(.+?)\s*<)?([^<>\s]+@[^<>\s]+)>?$/);
      if (!match) return { email: part };
      return { name: match[1]?.trim() || null, email: match[2] };
    });
}

function plainBody(message: MailMessageDetail | null): string {
  if (!message) return "";
  if (message.text) return message.text;
  if (!message.html) return "";
  if (typeof DOMParser === "undefined") return "";
  const document = new DOMParser().parseFromString(message.html, "text/html");
  return document.body.textContent?.trim() || "";
}

function replySubject(value: string): string {
  return /^re:/i.test(value) ? value : `Re: ${value}`;
}

function forwardSubject(value: string): string {
  return /^fwd?:/i.test(value) ? value : `Fwd: ${value}`;
}

function sizeLabel(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.ceil(bytes / 1024)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function MailComposer({
  open,
  onOpenChange,
  mode,
  sourceMessage,
  draft,
  identities,
  onSend,
  onSaveDraft,
  onSendDraft,
  onListAttachments,
  onUploadAttachment,
  onDeleteAttachment,
}: {
  open: boolean;
  onOpenChange(open: boolean): void;
  mode: ComposeMode;
  sourceMessage: MailMessageDetail | null;
  draft: MailDraft | null;
  identities: MailIdentity[];
  onSend(input: ComposeMailInput): Promise<void>;
  onSaveDraft(input: MailDraftInput, id?: string | null): Promise<MailDraft>;
  onSendDraft(id: string): Promise<unknown>;
  onListAttachments(id: string): Promise<MailDraftAttachment[]>;
  onUploadAttachment(id: string, file: File): Promise<MailDraftAttachment>;
  onDeleteAttachment(draftId: string, attachmentId: string): Promise<void>;
}) {
  const [identityId, setIdentityId] = useState("");
  const [to, setTo] = useState("");
  const [cc, setCc] = useState("");
  const [bcc, setBcc] = useState("");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");
  const [activeDraftId, setActiveDraftId] = useState<string | null>(null);
  const [attachments, setAttachments] = useState<MailDraftAttachment[]>([]);
  const [busy, setBusy] = useState<"save" | "send" | "attachment" | "">("");
  const [error, setError] = useState("");
  const fileInputRef = useRef<HTMLInputElement>(null);

  const selectedIdentity = useMemo(
    () => identities.find((identity) => identity.id === identityId) ?? null,
    [identities, identityId],
  );

  useEffect(() => {
    if (!open) return;
    setError("");
    setBusy("");

    if (mode === "draft" && draft) {
      setActiveDraftId(draft.id);
      setIdentityId(
        draft.identityId
          ?? identities.find((item) => item.accountId === draft.accountId && item.isDefault)?.id
          ?? identities.find((item) => item.accountId === draft.accountId)?.id
          ?? "",
      );
      setTo(addressesText(draft.to));
      setCc(addressesText(draft.cc));
      setBcc(addressesText(draft.bcc));
      setSubject(draft.subject);
      setBody(draft.bodyText);
      void onListAttachments(draft.id)
        .then(setAttachments)
        .catch((cause) => setError(cause instanceof Error ? cause.message : "无法读取草稿附件"));
      return;
    }

    setActiveDraftId(null);
    setAttachments([]);
    const preferred = sourceMessage
      ? identities.find((item) => item.accountId === sourceMessage.accountId && item.isDefault)
        ?? identities.find((item) => item.accountId === sourceMessage.accountId)
      : identities.find((item) => item.isDefault) ?? identities[0];
    setIdentityId(preferred?.id ?? "");
    setBcc("");

    if (mode === "reply" && sourceMessage) {
      setTo(addressesText(sourceMessage.replyTo?.length ? sourceMessage.replyTo : sourceMessage.from));
      setCc("");
      setSubject(replySubject(sourceMessage.subject));
      setBody("");
    } else if (mode === "replyAll" && sourceMessage) {
      setTo(addressesText(sourceMessage.replyTo?.length ? sourceMessage.replyTo : sourceMessage.from));
      setCc(addressesText(sourceMessage.to));
      setSubject(replySubject(sourceMessage.subject));
      setBody("");
    } else if (mode === "forward" && sourceMessage) {
      setTo("");
      setCc("");
      setSubject(forwardSubject(sourceMessage.subject));
      setBody(
        `\n\n---------- Forwarded message ----------\nFrom: ${addressesText(sourceMessage.from)}\nDate: ${sourceMessage.sentAt}\nSubject: ${sourceMessage.subject}\n\n${plainBody(sourceMessage)}`,
      );
    } else {
      setTo("");
      setCc("");
      setSubject("");
      setBody("");
    }
  }, [draft?.id, mode, open, sourceMessage?.id]);

  function draftInput(): MailDraftInput | null {
    if (!selectedIdentity) return null;
    return {
      accountId: selectedIdentity.accountId,
      identityId: selectedIdentity.id,
      inReplyToMessageId: mode === "reply" || mode === "replyAll"
        ? sourceMessage?.id ?? null
        : draft?.inReplyToMessageId ?? null,
      to: parseAddresses(to),
      cc: parseAddresses(cc),
      bcc: parseAddresses(bcc),
      subject: subject.trim(),
      bodyText: body,
    };
  }

  async function persistDraft(): Promise<MailDraft | null> {
    const input = draftInput();
    if (!input) {
      setError("请选择有效发件身份");
      return null;
    }
    const saved = await onSaveDraft(input, activeDraftId);
    setActiveDraftId(saved.id);
    return saved;
  }

  async function saveDraft() {
    setBusy("save");
    setError("");
    try {
      await persistDraft();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "保存草稿失败");
    } finally {
      setBusy("");
    }
  }

  async function uploadAttachment(file: File) {
    if (file.size <= 0) return;
    if (file.size > MAX_ATTACHMENT_BYTES) {
      setError("单封邮件附件总大小上限为 18 MB");
      return;
    }
    const existingBytes = attachments.reduce((sum, item) => sum + item.sizeBytes, 0);
    if (existingBytes + file.size > MAX_ATTACHMENT_BYTES) {
      setError("单封邮件附件总大小上限为 18 MB");
      return;
    }

    setBusy("attachment");
    setError("");
    try {
      const saved = await persistDraft();
      if (!saved) return;
      const uploaded = await onUploadAttachment(saved.id, file);
      setAttachments((items) => [...items, uploaded]);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "上传附件失败");
    } finally {
      setBusy("");
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  }

  async function removeAttachment(item: MailDraftAttachment) {
    if (!activeDraftId) return;
    setBusy("attachment");
    setError("");
    try {
      await onDeleteAttachment(activeDraftId, item.id);
      setAttachments((items) => items.filter((candidate) => candidate.id !== item.id));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "删除附件失败");
    } finally {
      setBusy("");
    }
  }

  async function sendMessage() {
    const input = draftInput();
    if (!input || !input.identityId) {
      setError("请选择有效发件身份");
      return;
    }
    if (!input.to.length || input.to.some((address) => !address.email.includes("@"))) {
      setError("请输入有效收件人");
      return;
    }

    setBusy("send");
    setError("");
    try {
      if (activeDraftId || attachments.length) {
        const saved = await persistDraft();
        if (!saved) return;
        await onSendDraft(saved.id);
      } else {
        await onSend({
          identityId: input.identityId,
          to: input.to,
          cc: input.cc,
          bcc: input.bcc,
          subject: input.subject,
          text: input.bodyText,
          inReplyToMessageId: input.inReplyToMessageId,
        });
      }
      onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "邮件发送失败");
    } finally {
      setBusy("");
    }
  }

  return <Dialog
    open={open}
    onOpenChange={onOpenChange}
    title={mode === "draft" ? "编辑草稿" : mode === "forward" ? "转发邮件" : mode === "reply" || mode === "replyAll" ? "回复邮件" : "写邮件"}
    description="只有点击“发送”才会投递邮件。"
  >
    <div className="space-y-3">
      <label className="block space-y-1.5 text-xs font-medium">发件身份
        <Select value={identityId} onChange={(event) => setIdentityId(event.target.value)}>
          <option value="">选择 Identity</option>
          {identities.map((identity) => <option key={identity.id} value={identity.id}>
            {identity.displayName ? `${identity.displayName} <${identity.email}>` : identity.email}
          </option>)}
        </Select>
      </label>
      <label className="block space-y-1.5 text-xs font-medium">收件人
        <Input value={to} onChange={(event) => setTo(event.target.value)} placeholder="name@example.com，多个地址可用逗号分隔" />
      </label>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block space-y-1.5 text-xs font-medium">Cc<Input value={cc} onChange={(event) => setCc(event.target.value)} /></label>
        <label className="block space-y-1.5 text-xs font-medium">Bcc<Input value={bcc} onChange={(event) => setBcc(event.target.value)} /></label>
      </div>
      <label className="block space-y-1.5 text-xs font-medium">主题<Input value={subject} onChange={(event) => setSubject(event.target.value)} /></label>
      <label className="block space-y-1.5 text-xs font-medium">正文
        <Textarea className="min-h-64 font-sans" value={body} onChange={(event) => setBody(event.target.value)} placeholder="输入邮件正文…" />
      </label>

      <section className="rounded-md border bg-muted/15 p-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-xs font-medium"><Paperclip size={13} />附件</div>
            <div className="mt-1 text-[10px] text-muted-foreground">总大小上限 18 MB。添加附件会自动建立服务端草稿。</div>
          </div>
          <input
            ref={fileInputRef}
            type="file"
            className="hidden"
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file) void uploadAttachment(file);
            }}
          />
          <Button size="sm" variant="outline" disabled={busy !== ""} onClick={() => fileInputRef.current?.click()}>
            {busy === "attachment" ? <Loader2 size={13} className="animate-spin" /> : <Paperclip size={13} />}添加
          </Button>
        </div>
        {attachments.length ? <div className="mt-3 space-y-1">
          {attachments.map((item) => <div key={item.id} className="flex items-center gap-2 rounded-md bg-card px-2.5 py-2 text-xs">
            <Paperclip size={12} className="shrink-0 text-muted-foreground" />
            <span className="min-w-0 flex-1 truncate">{item.filename}</span>
            <span className="shrink-0 text-[10px] text-muted-foreground">{sizeLabel(item.sizeBytes)}</span>
            <button className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-destructive" onClick={() => void removeAttachment(item)} aria-label={`删除 ${item.filename}`}><Trash2 size={12} /></button>
          </div>)}
        </div> : null}
      </section>

      {activeDraftId ? <div className="flex items-center gap-2 text-[11px] text-muted-foreground"><FileText size={12} />草稿已建立，可继续保存、添加附件或直接发送。</div> : null}
      {error ? <div className="rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive">{error}</div> : null}

      <div className="flex flex-wrap justify-end gap-2 border-t pt-3">
        <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={busy !== ""}>关闭</Button>
        <Button variant="outline" onClick={() => void saveDraft()} disabled={busy !== ""}>
          {busy === "save" ? <Loader2 size={15} className="animate-spin" /> : <Save size={15} />}
          {busy === "save" ? "保存中" : "保存草稿"}
        </Button>
        <Button onClick={() => void sendMessage()} disabled={busy !== ""}>
          {busy === "send" ? <Loader2 size={15} className="animate-spin" /> : <Send size={15} />}
          {busy === "send" ? "发送中" : "发送"}
        </Button>
      </div>
    </div>
  </Dialog>;
}
