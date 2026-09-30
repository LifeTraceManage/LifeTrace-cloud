import type { MailAddress, MailMessageSummary } from "./types";

const PUBLIC_MAIL_DOMAINS = new Set([
  "126.com",
  "163.com",
  "foxmail.com",
  "gmail.com",
  "googlemail.com",
  "hotmail.com",
  "icloud.com",
  "live.com",
  "mail.com",
  "outlook.com",
  "proton.me",
  "protonmail.com",
  "qq.com",
  "yeah.net",
  "yahoo.com",
  "yahoo.com.cn",
]);

export interface MailSourceIdentity {
  key: string;
  label: string;
  email: string | null;
  domain: string | null;
  groupedBy: "address" | "domain" | "unknown";
}

export interface MailSourceGroup extends MailSourceIdentity {
  messageCount: number;
  unreadCount: number;
  latestMessage: MailMessageSummary;
  messages: MailMessageSummary[];
}

function primarySender(message: MailMessageSummary): MailAddress | null {
  return message.from.find((item) => item.email.trim().includes("@")) ?? null;
}

function senderParts(message: MailMessageSummary): {
  sender: MailAddress | null;
  email: string | null;
  domain: string | null;
} {
  const sender = primarySender(message);
  const email = sender?.email.trim().toLocaleLowerCase() || null;
  const separator = email?.lastIndexOf("@") ?? -1;
  const domain = separator > 0 && separator < (email?.length ?? 0) - 1
    ? email!.slice(separator + 1)
    : null;
  return { sender, email, domain };
}

function senderLabel(sender: MailAddress | null, fallback: string): string {
  const name = sender?.name?.trim();
  return name || fallback;
}

export function mailSourceIdentity(message: MailMessageSummary): MailSourceIdentity {
  const { sender, email, domain } = senderParts(message);
  if (!email || !domain) {
    return {
      key: `unknown:${message.id}`,
      label: senderLabel(sender, "未知发件人"),
      email,
      domain,
      groupedBy: "unknown",
    };
  }

  if (PUBLIC_MAIL_DOMAINS.has(domain)) {
    return {
      key: `address:${email}`,
      label: senderLabel(sender, email),
      email,
      domain,
      groupedBy: "address",
    };
  }

  return {
    key: `domain:${domain}`,
    label: senderLabel(sender, domain),
    email,
    domain,
    groupedBy: "domain",
  };
}

function timeValue(message: MailMessageSummary): number {
  const value = new Date(message.sentAt).getTime();
  return Number.isFinite(value) ? value : 0;
}

export function groupMessagesBySource(messages: MailMessageSummary[]): MailSourceGroup[] {
  const groups = new Map<string, MailSourceGroup>();

  for (const message of messages) {
    const identity = mailSourceIdentity(message);
    const current = groups.get(identity.key);
    if (!current) {
      groups.set(identity.key, {
        ...identity,
        messageCount: 1,
        unreadCount: message.isRead ? 0 : 1,
        latestMessage: message,
        messages: [message],
      });
      continue;
    }

    current.messageCount += 1;
    if (!message.isRead) current.unreadCount += 1;
    current.messages.push(message);
    if (timeValue(message) > timeValue(current.latestMessage)) {
      current.latestMessage = message;
      current.label = identity.label;
      current.email = identity.email;
    }
  }

  return Array.from(groups.values())
    .map((group) => ({
      ...group,
      messages: [...group.messages].sort((left, right) => timeValue(right) - timeValue(left)),
    }))
    .sort((left, right) => timeValue(right.latestMessage) - timeValue(left.latestMessage));
}
