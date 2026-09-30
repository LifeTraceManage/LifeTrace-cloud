import { describe, expect, it } from "vitest";

import { groupMessagesBySource, mailSourceIdentity } from "./mailSource";
import type { MailMessageSummary } from "./types";

function message(
  id: string,
  email: string,
  sentAt: string,
  options: { name?: string; isRead?: boolean; subject?: string } = {},
): MailMessageSummary {
  return {
    id,
    accountId: "account-1",
    subject: options.subject ?? id,
    from: [{ email, name: options.name }],
    sentAt,
    isRead: options.isRead ?? true,
    isStarred: false,
  };
}

describe("mail source grouping", () => {
  it("groups service senders by domain", () => {
    const first = message("m1", "notifications@github.com", "2026-09-30T08:00:00Z", { name: "GitHub" });
    const second = message("m2", "noreply@github.com", "2026-09-30T09:00:00Z", { name: "GitHub Actions", isRead: false });

    const groups = groupMessagesBySource([first, second]);

    expect(groups).toHaveLength(1);
    expect(groups[0].key).toBe("domain:github.com");
    expect(groups[0].messageCount).toBe(2);
    expect(groups[0].unreadCount).toBe(1);
    expect(groups[0].latestMessage.id).toBe("m2");
  });

  it("does not merge unrelated users on public mailbox domains", () => {
    const groups = groupMessagesBySource([
      message("m1", "alice@gmail.com", "2026-09-30T08:00:00Z"),
      message("m2", "bob@gmail.com", "2026-09-30T09:00:00Z"),
    ]);

    expect(groups).toHaveLength(2);
    expect(groups.map((item) => item.key).sort()).toEqual([
      "address:alice@gmail.com",
      "address:bob@gmail.com",
    ]);
  });

  it("still groups repeated mail from the same public mailbox address", () => {
    const groups = groupMessagesBySource([
      message("m1", "alice@qq.com", "2026-09-30T08:00:00Z"),
      message("m2", "alice@qq.com", "2026-09-30T09:00:00Z"),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0].messageCount).toBe(2);
  });

  it("falls back to an isolated source when sender information is missing", () => {
    const source = mailSourceIdentity({
      ...message("m1", "placeholder@example.com", "2026-09-30T08:00:00Z"),
      from: [],
    });

    expect(source.key).toBe("unknown:m1");
    expect(source.groupedBy).toBe("unknown");
  });
});
