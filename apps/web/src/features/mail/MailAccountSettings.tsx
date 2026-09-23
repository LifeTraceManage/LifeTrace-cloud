import { useMemo, useState, type FormEvent } from "react";
import { CheckCircle2, Edit3, Loader2, MailPlus, RefreshCw, Settings, Trash2, UserPlus, X } from "lucide-react";
import { Badge, Button, Dialog, Input, Select, Textarea } from "../../components/ui";
import type {
  MailAccount,
  MailAccountInput,
  MailConnectionTest,
  MailIdentity,
  MailIdentityInput,
} from "./types";

const providerOptions = [
  ["qq", "QQ Mail"],
  ["163", "163 Mail"],
  ["126", "126 Mail"],
  ["yeah", "Yeah Mail"],
  ["generic", "Generic IMAP / SMTP"],
] as const;

function statusLabel(status: string): string {
  return ({
    active: "已连接",
    degraded: "部分可用",
    validating: "验证中",
    disabled: "已停用",
  } as Record<string, string>)[status] ?? status;
}

export function MailAccountSettings({
  open,
  onOpenChange,
  accounts,
  identities,
  onConnect,
  onDisconnect,
  onTest,
  onSync,
  onCreateIdentity,
  onUpdateIdentity,
  onDeleteIdentity,
}: {
  open: boolean;
  onOpenChange(open: boolean): void;
  accounts: MailAccount[];
  identities: MailIdentity[];
  onConnect(input: MailAccountInput): Promise<MailAccount>;
  onDisconnect(id: string): Promise<void>;
  onTest(id: string): Promise<MailConnectionTest>;
  onSync(id: string): Promise<{ ok: boolean; syncedMessages: number }>;
  onCreateIdentity(input: MailIdentityInput): Promise<MailIdentity>;
  onUpdateIdentity(id: string, input: MailIdentityInput): Promise<MailIdentity>;
  onDeleteIdentity(id: string): Promise<void>;
}) {
  const [provider, setProvider] = useState<MailAccountInput["provider"]>("qq");
  const [emailAddress, setEmailAddress] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [username, setUsername] = useState("");
  const [authorizationCode, setAuthorizationCode] = useState("");
  const [imapHost, setImapHost] = useState("");
  const [imapPort, setImapPort] = useState("993");
  const [imapSecurity, setImapSecurity] = useState<"tls" | "starttls">("tls");
  const [smtpHost, setSmtpHost] = useState("");
  const [smtpPort, setSmtpPort] = useState("465");
  const [smtpSecurity, setSmtpSecurity] = useState<"tls" | "starttls">("tls");
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");

  const [editingIdentityId, setEditingIdentityId] = useState<string | null>(null);
  const [identityAccountId, setIdentityAccountId] = useState("");
  const [identityEmail, setIdentityEmail] = useState("");
  const [identityName, setIdentityName] = useState("");
  const [identityReplyTo, setIdentityReplyTo] = useState("");
  const [identitySignature, setIdentitySignature] = useState("");
  const [identityDefault, setIdentityDefault] = useState(false);

  const effectiveIdentityAccountId = identityAccountId || accounts[0]?.id || "";
  const identityGroups = useMemo(() => new Map(accounts.map((account) => [
    account.id,
    identities.filter((identity) => identity.accountId === account.id),
  ])), [accounts, identities]);

  function resetIdentityForm(accountId = accounts[0]?.id ?? "") {
    setEditingIdentityId(null);
    setIdentityAccountId(accountId);
    setIdentityEmail("");
    setIdentityName("");
    setIdentityReplyTo("");
    setIdentitySignature("");
    setIdentityDefault(false);
  }

  function editIdentity(identity: MailIdentity) {
    setEditingIdentityId(identity.id);
    setIdentityAccountId(identity.accountId);
    setIdentityEmail(identity.email);
    setIdentityName(identity.displayName ?? "");
    setIdentityReplyTo(identity.replyTo ?? "");
    setIdentitySignature(identity.signatureHtml ?? "");
    setIdentityDefault(identity.isDefault);
  }

  async function connect(event: FormEvent) {
    event.preventDefault();
    setBusy("connect");
    setNotice("");
    setError("");
    try {
      await onConnect({
        provider,
        emailAddress: emailAddress.trim(),
        displayName: displayName.trim() || null,
        username: username.trim() || null,
        authorizationCode,
        ...(provider === "generic" ? {
          imapHost: imapHost.trim(),
          imapPort: Number(imapPort),
          imapSecurity,
          smtpHost: smtpHost.trim(),
          smtpPort: Number(smtpPort),
          smtpSecurity,
        } : {}),
      });
      setAuthorizationCode("");
      setNotice("邮箱账号已保存，连接测试会由 LifeTrace Cloud 执行。");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "邮箱连接失败");
    } finally {
      setBusy("");
    }
  }

  async function runAccountAction(id: string, action: "test" | "sync" | "disconnect") {
    setBusy(`${action}:${id}`);
    setNotice("");
    setError("");
    try {
      if (action === "test") {
        const result = await onTest(id);
        setNotice(`连接测试：IMAP ${result.imapOk ? "正常" : "失败"}，SMTP ${result.smtpOk ? "正常" : "失败"}，发现 ${result.folders.length} 个文件夹。`);
      } else if (action === "sync") {
        const result = await onSync(id);
        setNotice(`同步完成，本轮写入/更新 ${result.syncedMessages} 封邮件。`);
      } else {
        await onDisconnect(id);
        if (identityAccountId === id) resetIdentityForm();
        setNotice("邮箱账号已断开，凭据、Identity 与未发送草稿已撤销。");
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "邮箱操作失败");
    } finally {
      setBusy("");
    }
  }

  async function saveIdentity(event: FormEvent) {
    event.preventDefault();
    if (!effectiveIdentityAccountId) return;
    setBusy("identity");
    setNotice("");
    setError("");
    const input: MailIdentityInput = {
      accountId: effectiveIdentityAccountId,
      emailAddress: identityEmail.trim(),
      displayName: identityName.trim() || null,
      replyTo: identityReplyTo.trim() || null,
      signatureHtml: identitySignature.trim() || null,
      isDefault: identityDefault,
    };
    try {
      if (editingIdentityId) {
        await onUpdateIdentity(editingIdentityId, input);
        setNotice("发件 Identity 已更新。签名会在服务端发送前自动附加。");
      } else {
        await onCreateIdentity(input);
        setNotice("发件 Identity 已添加。");
      }
      resetIdentityForm(effectiveIdentityAccountId);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "保存 Identity 失败");
    } finally {
      setBusy("");
    }
  }

  async function removeIdentity(identity: MailIdentity) {
    setBusy(`identity-delete:${identity.id}`);
    setNotice("");
    setError("");
    try {
      await onDeleteIdentity(identity.id);
      if (editingIdentityId === identity.id) resetIdentityForm(identity.accountId);
      setNotice("Identity 已删除；若删除的是默认 Identity，服务端会自动选择同账号下的下一个 Identity。");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "删除 Identity 失败");
    } finally {
      setBusy("");
    }
  }

  return <Dialog
    open={open}
    onOpenChange={onOpenChange}
    title="Mail 账号与 Identity"
    description="邮箱密码/授权码只提交给 LifeTrace Cloud，浏览器不会持久化 IMAP/SMTP 凭据。"
  >
    <div className="space-y-6">
      {notice ? <div className="rounded-md border border-success/30 bg-success/10 p-3 text-xs">{notice}</div> : null}
      {error ? <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">{error}</div> : null}

      <section>
        <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><Settings size={15} />已连接账号</div>
        <div className="space-y-2">
          {accounts.map((account) => {
            const group = identityGroups.get(account.id) ?? [];
            return <div key={account.id} className="rounded-md border bg-card p-3">
              <div className="flex items-start gap-3">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">{account.displayName || account.email}</div>
                  <div className="mt-0.5 truncate text-xs text-muted-foreground">{account.email} · {account.provider}</div>
                </div>
                <Badge>{statusLabel(account.status)}</Badge>
              </div>
              <div className="mt-2 text-[11px] text-muted-foreground">
                {account.lastSyncAt ? `上次同步：${new Date(account.lastSyncAt).toLocaleString("zh-CN")}` : "尚未完成首次同步"}
                {account.idleSupported ? " · IMAP IDLE" : ""}
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                <Button size="sm" variant="outline" disabled={busy !== ""} onClick={() => void runAccountAction(account.id, "test")}>
                  {busy === `test:${account.id}` ? <Loader2 size={13} className="animate-spin" /> : <CheckCircle2 size={13} />}测试
                </Button>
                <Button size="sm" variant="outline" disabled={busy !== ""} onClick={() => void runAccountAction(account.id, "sync")}>
                  {busy === `sync:${account.id}` ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}同步
                </Button>
                <Button size="sm" variant="ghost" disabled={busy !== ""} onClick={() => void runAccountAction(account.id, "disconnect")}>
                  <Trash2 size={13} />断开
                </Button>
              </div>
              <div className="mt-3 border-t pt-2">
                <div className="mb-1 text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">Identities</div>
                {group.map((identity) => <div key={identity.id} className="flex items-center gap-2 py-1 text-xs">
                  <span className="min-w-0 flex-1 truncate">{identity.displayName ? `${identity.displayName} <${identity.email}>` : identity.email}</span>
                  {identity.signatureHtml ? <Badge>签名</Badge> : null}
                  {identity.isDefault ? <Badge>默认</Badge> : null}
                  <button className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground" onClick={() => editIdentity(identity)} aria-label="编辑 Identity"><Edit3 size={12} /></button>
                  {group.length > 1 ? <button className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-destructive" onClick={() => void removeIdentity(identity)} aria-label="删除 Identity"><Trash2 size={12} /></button> : null}
                </div>)}
              </div>
            </div>;
          })}
          {!accounts.length ? <div className="rounded-md border border-dashed p-4 text-xs leading-5 text-muted-foreground">还没有邮箱账号。先在下面连接一个现有邮箱。</div> : null}
        </div>
      </section>

      <section className="border-t pt-5">
        <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><MailPlus size={15} />连接邮箱</div>
        <form className="space-y-3" onSubmit={(event) => void connect(event)}>
          <label className="block space-y-1 text-xs font-medium">Provider
            <Select value={provider} onChange={(event) => setProvider(event.target.value as MailAccountInput["provider"])}>
              {providerOptions.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
            </Select>
          </label>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="block space-y-1 text-xs font-medium">邮箱地址<Input type="email" required value={emailAddress} onChange={(event) => setEmailAddress(event.target.value)} /></label>
            <label className="block space-y-1 text-xs font-medium">显示名称<Input value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label>
          </div>
          <label className="block space-y-1 text-xs font-medium">登录用户名（留空则使用邮箱）<Input value={username} onChange={(event) => setUsername(event.target.value)} /></label>
          <label className="block space-y-1 text-xs font-medium">授权码 / 应用专用密码<Input type="password" required value={authorizationCode} onChange={(event) => setAuthorizationCode(event.target.value)} autoComplete="new-password" /></label>

          {provider === "generic" ? <div className="grid gap-3 rounded-md border bg-muted/20 p-3 sm:grid-cols-2">
            <label className="block space-y-1 text-xs font-medium">IMAP Host<Input required value={imapHost} onChange={(event) => setImapHost(event.target.value)} /></label>
            <label className="block space-y-1 text-xs font-medium">IMAP Port<Input type="number" required value={imapPort} onChange={(event) => setImapPort(event.target.value)} /></label>
            <label className="block space-y-1 text-xs font-medium">IMAP Security<Select value={imapSecurity} onChange={(event) => setImapSecurity(event.target.value as "tls" | "starttls")}><option value="tls">TLS</option><option value="starttls">STARTTLS</option></Select></label>
            <label className="block space-y-1 text-xs font-medium">SMTP Host<Input required value={smtpHost} onChange={(event) => setSmtpHost(event.target.value)} /></label>
            <label className="block space-y-1 text-xs font-medium">SMTP Port<Input type="number" required value={smtpPort} onChange={(event) => setSmtpPort(event.target.value)} /></label>
            <label className="block space-y-1 text-xs font-medium">SMTP Security<Select value={smtpSecurity} onChange={(event) => setSmtpSecurity(event.target.value as "tls" | "starttls")}><option value="tls">TLS</option><option value="starttls">STARTTLS</option></Select></label>
          </div> : null}

          <Button type="submit" disabled={busy !== ""}>{busy === "connect" ? <Loader2 size={14} className="animate-spin" /> : <MailPlus size={14} />}连接邮箱</Button>
        </form>
      </section>

      {accounts.length ? <section className="border-t pt-5">
        <div className="mb-3 flex items-center justify-between gap-3">
          <div className="flex items-center gap-2 text-sm font-semibold"><UserPlus size={15} />{editingIdentityId ? "编辑发件 Identity" : "添加发件 Identity"}</div>
          {editingIdentityId ? <Button size="sm" variant="ghost" onClick={() => resetIdentityForm(effectiveIdentityAccountId)}><X size={13} />取消编辑</Button> : null}
        </div>
        <form className="space-y-3" onSubmit={(event) => void saveIdentity(event)}>
          <label className="block space-y-1 text-xs font-medium">所属账号
            <Select disabled={Boolean(editingIdentityId)} value={effectiveIdentityAccountId} onChange={(event) => setIdentityAccountId(event.target.value)}>
              {accounts.map((account) => <option key={account.id} value={account.id}>{account.email}</option>)}
            </Select>
          </label>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="block space-y-1 text-xs font-medium">From 地址<Input type="email" required value={identityEmail} onChange={(event) => setIdentityEmail(event.target.value)} /></label>
            <label className="block space-y-1 text-xs font-medium">显示名称<Input value={identityName} onChange={(event) => setIdentityName(event.target.value)} /></label>
          </div>
          <label className="block space-y-1 text-xs font-medium">Reply-To<Input type="email" value={identityReplyTo} onChange={(event) => setIdentityReplyTo(event.target.value)} /></label>
          <label className="block space-y-1 text-xs font-medium">签名（纯文本）
            <Textarea className="min-h-20" value={identitySignature} onChange={(event) => setIdentitySignature(event.target.value)} placeholder="发送前由 LifeTrace Cloud 自动附加到正文末尾" />
          </label>
          <label className="flex items-center gap-2 text-xs">
            <input type="checkbox" checked={identityDefault} onChange={(event) => setIdentityDefault(event.target.checked)} />
            设为该邮箱账号的默认发件 Identity
          </label>
          <Button type="submit" variant="outline" disabled={busy !== ""}>{busy === "identity" ? <Loader2 size={14} className="animate-spin" /> : editingIdentityId ? <Edit3 size={14} /> : <UserPlus size={14} />}{editingIdentityId ? "保存 Identity" : "添加 Identity"}</Button>
        </form>
      </section> : null}
    </div>
  </Dialog>;
}
