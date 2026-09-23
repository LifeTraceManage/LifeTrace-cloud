export type MailRuntimeStatus = "ready" | "unavailable";

export interface MailRuntimeInfo {
  status: MailRuntimeStatus;
  provider: "lifetrace-cloud";
  lastCheckedAt: string;
}

export interface MailAccount {
  id: string;
  provider: string;
  email: string;
  displayName?: string | null;
  status: string;
  idleSupported?: boolean;
  lastValidatedAt?: string | null;
  lastSyncAt?: string | null;
  lastErrorCode?: string | null;
}

export interface MailAccountInput {
  provider: "qq" | "163" | "126" | "yeah" | "generic";
  emailAddress: string;
  displayName?: string | null;
  username?: string | null;
  authorizationCode: string;
  imapHost?: string | null;
  imapPort?: number | null;
  imapSecurity?: "tls" | "starttls" | null;
  smtpHost?: string | null;
  smtpPort?: number | null;
  smtpSecurity?: "tls" | "starttls" | null;
}

export interface MailIdentity {
  id: string;
  accountId: string;
  email: string;
  displayName?: string | null;
  replyTo?: string | null;
  signatureHtml?: string | null;
  isDefault: boolean;
}

export interface MailIdentityInput {
  accountId: string;
  emailAddress: string;
  displayName?: string | null;
  replyTo?: string | null;
  signatureHtml?: string | null;
  isDefault?: boolean;
}

export interface Mailbox {
  id: string;
  accountId: string;
  name: string;
  role: "inbox" | "sent" | "drafts" | "archive" | "trash" | "spam" | "other" | string;
  lastSyncAt?: string | null;
  syncEnabled?: boolean;
}

export interface MailAddress {
  name?: string | null;
  email: string;
}

export interface MailAttachment {
  id: string;
  messageId: string;
  filename: string;
  contentType?: string | null;
  sizeBytes?: number | null;
  downloadUrl?: string | null;
}

export interface MailMessageSummary {
  id: string;
  threadId?: string | null;
  accountId: string;
  mailboxId?: string | null;
  subject: string;
  preview?: string | null;
  from: MailAddress[];
  to?: MailAddress[];
  sentAt: string;
  isRead: boolean;
  isStarred: boolean;
  isArchived?: boolean;
  hasAttachments?: boolean;
}

export interface MailMessageDetail extends MailMessageSummary {
  cc?: MailAddress[];
  bcc?: MailAddress[];
  replyTo?: MailAddress[];
  html?: string | null;
  text?: string | null;
  attachments: MailAttachment[];
}

export interface MailMessagePage {
  items: MailMessageSummary[];
  hasMore: boolean;
  nextOffset: number;
}

export interface MailDraft {
  id: string;
  accountId: string;
  identityId?: string | null;
  inReplyToMessageId?: string | null;
  to: MailAddress[];
  cc: MailAddress[];
  bcc: MailAddress[];
  subject: string;
  bodyText: string;
  state: string;
  createdAt: string;
  updatedAt: string;
}

export interface MailDraftAttachment {
  id: string;
  draftId: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
}

export interface MailDraftInput {
  accountId: string;
  identityId?: string | null;
  inReplyToMessageId?: string | null;
  to: MailAddress[];
  cc?: MailAddress[];
  bcc?: MailAddress[];
  subject: string;
  bodyText: string;
}

export interface ComposeMailInput {
  identityId: string;
  to: MailAddress[];
  cc?: MailAddress[];
  bcc?: MailAddress[];
  subject: string;
  text?: string;
  inReplyToMessageId?: string | null;
}

export interface MailListQuery {
  accountId?: string | null;
  mailboxRole?: string | null;
  mailboxId?: string | null;
  query?: string;
  unreadOnly?: boolean | null;
  starredOnly?: boolean | null;
  offset?: number;
  limit?: number;
}

export interface MailConnectionTest {
  imapOk: boolean;
  smtpOk: boolean;
  idleSupported: boolean;
  folders: string[];
}
