import { API_BASE, browserFetch } from "../../services/core";
import type {
  ComposeMailInput,
  MailAccount,
  MailAccountInput,
  MailAttachment,
  MailConnectionTest,
  MailDraft,
  MailDraftAttachment,
  MailDraftInput,
  MailIdentity,
  MailIdentityInput,
  MailListQuery,
  MailMessageDetail,
  MailMessagePage,
  MailMessageSummary,
  MailRuntimeInfo,
  Mailbox,
  MailAddress,
} from "./types";

function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

async function readError(response: Response): Promise<string> {
  const raw = await response.text();
  if (!raw) return `Mail API ${response.status}`;
  try {
    const payload = JSON.parse(raw) as { message?: string; error?: { message?: string } };
    return payload.message || payload.error?.message || `Mail API ${response.status}`;
  } catch {
    return raw;
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function addresses(value: unknown): MailAddress[] {
  const result: MailAddress[] = [];
  const seen = new Set<string>();

  const visit = (candidate: unknown) => {
    if (typeof candidate === "string") {
      const email = candidate.trim();
      if (email.includes("@") && !seen.has(email.toLocaleLowerCase())) {
        seen.add(email.toLocaleLowerCase());
        result.push({ email });
      }
      return;
    }
    if (Array.isArray(candidate)) {
      candidate.forEach(visit);
      return;
    }
    const record = asRecord(candidate);
    if (!record) return;
    const emailValue = typeof record.email === "string"
      ? record.email
      : typeof record.address === "string"
        ? record.address
        : null;
    if (emailValue?.includes("@")) {
      const email = emailValue.trim();
      const key = email.toLocaleLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        result.push({
          email,
          name: typeof record.name === "string" && record.name.trim() ? record.name.trim() : null,
        });
      }
    }
    for (const nested of Object.values(record)) {
      if (nested !== emailValue && nested !== record.name) visit(nested);
    }
  };

  visit(value);
  return result;
}

function isStarred(flags: unknown): boolean {
  if (!Array.isArray(flags)) return false;
  return flags.some((flag) =>
    typeof flag === "string"
    && (flag.toLocaleLowerCase() === "\\flagged" || flag.toLocaleLowerCase() === "flagged")
  );
}

interface RawAccount {
  id: string;
  provider: string;
  emailAddress: string;
  displayName?: string | null;
  status: string;
  idleSupported?: boolean;
  lastValidatedAt?: string | null;
  lastSyncAt?: string | null;
  lastErrorCode?: string | null;
}

interface RawIdentity {
  id: string;
  accountId: string;
  emailAddress: string;
  displayName?: string | null;
  replyTo?: string | null;
  signatureHtml?: string | null;
  isDefault: boolean;
}

interface RawFolder {
  id: string;
  accountId: string;
  remoteName: string;
  normalizedRole: string;
  lastSyncAt?: string | null;
  syncEnabled?: boolean;
}

interface RawMessage {
  id: string;
  accountId: string;
  folderId: string;
  threadId?: string | null;
  subject: string;
  fromJson?: unknown;
  toJson?: unknown;
  ccJson?: unknown;
  bccJson?: unknown;
  replyToJson?: unknown;
  sentAt?: string | null;
  receivedAt: string;
  flagsJson?: unknown;
  isRead: boolean;
  isArchived?: boolean;
  isStarred?: boolean;
  snippet?: string | null;
  bodyText?: string | null;
  bodyHtmlSanitized?: string | null;
  hasAttachments?: boolean;
}

interface RawAttachment {
  id: string;
  messageId: string;
  filename?: string | null;
  mimeType?: string | null;
  sizeBytes?: number | null;
  contentId?: string | null;
  disposition?: string | null;
}

interface RawDraft {
  id: string;
  accountId: string;
  identityId?: string | null;
  inReplyToMessageId?: string | null;
  toJson?: unknown;
  ccJson?: unknown;
  bccJson?: unknown;
  subject: string;
  bodyText: string;
  state: string;
  createdAt: string;
  updatedAt: string;
}

function account(raw: RawAccount): MailAccount {
  return {
    id: raw.id,
    provider: raw.provider,
    email: raw.emailAddress,
    displayName: raw.displayName,
    status: raw.status,
    idleSupported: raw.idleSupported,
    lastValidatedAt: raw.lastValidatedAt,
    lastSyncAt: raw.lastSyncAt,
    lastErrorCode: raw.lastErrorCode,
  };
}

function identity(raw: RawIdentity): MailIdentity {
  return {
    id: raw.id,
    accountId: raw.accountId,
    email: raw.emailAddress,
    displayName: raw.displayName,
    replyTo: raw.replyTo,
    signatureHtml: raw.signatureHtml,
    isDefault: raw.isDefault,
  };
}

function mailbox(raw: RawFolder): Mailbox {
  return {
    id: raw.id,
    accountId: raw.accountId,
    name: raw.remoteName,
    role: raw.normalizedRole,
    lastSyncAt: raw.lastSyncAt,
    syncEnabled: raw.syncEnabled,
  };
}

function summary(raw: RawMessage): MailMessageSummary {
  return {
    id: raw.id,
    threadId: raw.threadId,
    accountId: raw.accountId,
    mailboxId: raw.folderId,
    subject: raw.subject,
    preview: raw.snippet,
    from: addresses(raw.fromJson),
    to: addresses(raw.toJson),
    sentAt: raw.sentAt || raw.receivedAt,
    isRead: raw.isRead,
    isStarred: raw.isStarred ?? isStarred(raw.flagsJson),
    isArchived: raw.isArchived,
    hasAttachments: raw.hasAttachments,
  };
}

function draft(raw: RawDraft): MailDraft {
  return {
    id: raw.id,
    accountId: raw.accountId,
    identityId: raw.identityId,
    inReplyToMessageId: raw.inReplyToMessageId,
    to: addresses(raw.toJson),
    cc: addresses(raw.ccJson),
    bcc: addresses(raw.bccJson),
    subject: raw.subject,
    bodyText: raw.bodyText,
    state: raw.state,
    createdAt: raw.createdAt,
    updatedAt: raw.updatedAt,
  };
}

function addressStrings(values?: MailAddress[]): string[] {
  return (values ?? []).map((value) => value.name
    ? `${value.name} <${value.email}>`
    : value.email);
}

export class MailApi {
  constructor(private readonly csrfToken?: string) {}

  private async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const method = (init.method ?? "GET").toUpperCase();
    const headers = new Headers(init.headers);
    const isFormData = typeof FormData !== "undefined" && init.body instanceof FormData;
    if (init.body && !isFormData && !headers.has("content-type")) headers.set("content-type", "application/json");
    if (!["GET", "HEAD", "OPTIONS"].includes(method) && this.csrfToken) {
      headers.set("x-csrf-token", this.csrfToken);
    }
    const response = await browserFetch(apiUrl(path), {
      ...init,
      method,
      headers,
      credentials: "include",
    });
    if (!response.ok) throw new Error(await readError(response));
    if (response.status === 204) return undefined as T;
    return response.json() as Promise<T>;
  }

  async runtime(): Promise<MailRuntimeInfo> {
    await this.accounts();
    return {
      status: "ready",
      provider: "lifetrace-cloud",
      lastCheckedAt: new Date().toISOString(),
    };
  }

  async accounts(): Promise<MailAccount[]> {
    const value = await this.request<{ items: RawAccount[] }>("/api/v1/mail/accounts");
    return value.items.map(account);
  }

  createAccount(input: MailAccountInput): Promise<MailAccount> {
    return this.request<RawAccount>("/api/v1/mail/accounts", {
      method: "POST",
      body: JSON.stringify(input),
    }).then(account);
  }

  disconnectAccount(accountId: string): Promise<void> {
    return this.request(`/api/v1/mail/accounts/${encodeURIComponent(accountId)}`, {
      method: "DELETE",
      body: "{}",
    });
  }

  testAccount(accountId: string): Promise<MailConnectionTest> {
    return this.request(`/api/v1/mail/accounts/${encodeURIComponent(accountId)}/test`, {
      method: "POST",
      body: "{}",
    });
  }

  syncAccount(accountId: string): Promise<{ ok: boolean; syncedMessages: number }> {
    return this.request(`/api/v1/mail/accounts/${encodeURIComponent(accountId)}/sync`, {
      method: "POST",
      body: "{}",
    });
  }

  async identities(): Promise<MailIdentity[]> {
    const value = await this.request<{ items: RawIdentity[] }>("/api/v1/mail/identities");
    return value.items.map(identity);
  }

  createIdentity(input: MailIdentityInput): Promise<MailIdentity> {
    return this.request<RawIdentity>("/api/v1/mail/identities", {
      method: "POST",
      body: JSON.stringify(input),
    }).then(identity);
  }

  updateIdentity(id: string, input: MailIdentityInput): Promise<MailIdentity> {
    return this.request<RawIdentity>(`/api/v1/mail/identities/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }).then(identity);
  }

  deleteIdentity(id: string): Promise<void> {
    return this.request(`/api/v1/mail/identities/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: "{}",
    });
  }

  async mailboxes(accountId?: string | null): Promise<Mailbox[]> {
    if (accountId) {
      const value = await this.request<{ items: RawFolder[] }>(
        `/api/v1/mail/accounts/${encodeURIComponent(accountId)}/folders`
      );
      return value.items.map(mailbox);
    }
    const accounts = await this.accounts();
    const values = await Promise.all(accounts.map((item) => this.mailboxes(item.id)));
    return values.flat();
  }

  async messages(query: MailListQuery): Promise<MailMessagePage> {
    const params = new URLSearchParams();
    if (query.accountId) params.set("accountId", query.accountId);
    if (query.mailboxId) params.set("folderId", query.mailboxId);
    if (query.mailboxRole && query.mailboxRole !== "starred") params.set("role", query.mailboxRole);
    if (query.query?.trim()) params.set("q", query.query.trim());
    if (query.unreadOnly !== null && query.unreadOnly !== undefined) params.set("unreadOnly", String(query.unreadOnly));
    if (query.starredOnly || query.mailboxRole === "starred") params.set("starredOnly", "true");
    params.set("offset", String(query.offset ?? 0));
    params.set("limit", String(query.limit ?? 100));

    const value = await this.request<{ items: RawMessage[]; hasMore: boolean; nextOffset: number }>(
      `/api/v1/mail/messages?${params}`
    );
    return {
      items: value.items.map(summary),
      hasMore: value.hasMore,
      nextOffset: value.nextOffset,
    };
  }

  async message(messageId: string): Promise<MailMessageDetail> {
    const [raw, attachmentResponse] = await Promise.all([
      this.request<RawMessage>(`/api/v1/mail/messages/${encodeURIComponent(messageId)}`),
      this.request<{ items: RawAttachment[] }>(
        `/api/v1/mail/messages/${encodeURIComponent(messageId)}/attachments`
      ),
    ]);
    const base = summary(raw);
    const attachments: MailAttachment[] = attachmentResponse.items.map((item) => ({
      id: item.id,
      messageId: item.messageId,
      filename: item.filename || "attachment",
      contentType: item.mimeType,
      sizeBytes: item.sizeBytes,
      contentId: item.contentId,
      disposition: item.disposition,
      downloadUrl: apiUrl(`/api/v1/mail/attachments/${encodeURIComponent(item.id)}/content`),
    }));
    return {
      ...base,
      cc: addresses(raw.ccJson),
      bcc: addresses(raw.bccJson),
      replyTo: addresses(raw.replyToJson),
      html: raw.bodyHtmlSanitized,
      text: raw.bodyText,
      attachments,
    };
  }

  async send(input: ComposeMailInput): Promise<{ messageId: string }> {
    const identities = await this.identities();
    const selected = identities.find((item) => item.id === input.identityId);
    if (!selected) throw new Error("发件身份不存在或已失效");
    return this.request(`/api/v1/mail/accounts/${encodeURIComponent(selected.accountId)}/send`, {
      method: "POST",
      body: JSON.stringify({
        identityId: input.identityId,
        to: addressStrings(input.to),
        cc: addressStrings(input.cc),
        bcc: addressStrings(input.bcc),
        subject: input.subject,
        bodyText: input.text ?? "",
        inReplyToMessageId: input.inReplyToMessageId ?? null,
        idempotencyKey: crypto.randomUUID(),
      }),
    });
  }

  markRead(messageId: string, isRead: boolean): Promise<void> {
    return this.request(`/api/v1/mail/messages/${encodeURIComponent(messageId)}/read`, {
      method: "POST",
      body: JSON.stringify({ read: isRead }),
    });
  }

  setStarred(messageId: string, starred: boolean): Promise<void> {
    return this.request(`/api/v1/mail/messages/${encodeURIComponent(messageId)}/star`, {
      method: "POST",
      body: JSON.stringify({ starred }),
    });
  }

  move(messageId: string, destinationRole: "archive" | "trash" | "inbox"): Promise<void> {
    return this.request(`/api/v1/mail/messages/${encodeURIComponent(messageId)}/move`, {
      method: "POST",
      body: JSON.stringify({ destinationRole }),
    });
  }

  async drafts(): Promise<MailDraft[]> {
    const value = await this.request<{ items: RawDraft[] }>("/api/v1/mail/drafts");
    return value.items.map(draft);
  }

  createDraft(input: MailDraftInput): Promise<MailDraft> {
    return this.request<RawDraft>("/api/v1/mail/drafts", {
      method: "POST",
      body: JSON.stringify({
        ...input,
        to: addressStrings(input.to),
        cc: addressStrings(input.cc),
        bcc: addressStrings(input.bcc),
      }),
    }).then(draft);
  }

  updateDraft(id: string, input: MailDraftInput): Promise<MailDraft> {
    return this.request<RawDraft>(`/api/v1/mail/drafts/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body: JSON.stringify({
        ...input,
        to: addressStrings(input.to),
        cc: addressStrings(input.cc),
        bcc: addressStrings(input.bcc),
      }),
    }).then(draft);
  }

  deleteDraft(id: string): Promise<void> {
    return this.request(`/api/v1/mail/drafts/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: "{}",
    });
  }

  sendDraft(id: string): Promise<{ messageId: string }> {
    return this.request(`/api/v1/mail/drafts/${encodeURIComponent(id)}/send`, {
      method: "POST",
      body: "{}",
    });
  }


  async draftAttachments(draftId: string): Promise<MailDraftAttachment[]> {
    const value = await this.request<{ items: MailDraftAttachment[] }>(
      `/api/v1/mail/drafts/${encodeURIComponent(draftId)}/attachments`
    );
    return value.items;
  }

  uploadDraftAttachment(draftId: string, file: File): Promise<MailDraftAttachment> {
    const body = new FormData();
    body.append("file", file, file.name);
    return this.request(
      `/api/v1/mail/drafts/${encodeURIComponent(draftId)}/attachments`,
      { method: "POST", body },
    );
  }

  deleteDraftAttachment(draftId: string, attachmentId: string): Promise<void> {
    return this.request(
      `/api/v1/mail/drafts/${encodeURIComponent(draftId)}/attachments/${encodeURIComponent(attachmentId)}`,
      { method: "DELETE", body: "{}" },
    );
  }

}
