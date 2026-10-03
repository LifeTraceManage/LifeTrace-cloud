// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { MailMessageRow } from "./MailMessageRow";
import type { MailMessageSummary } from "./types";

function message(isRead: boolean): MailMessageSummary {
  return {
    id: "message-1",
    accountId: "account-1",
    mailboxId: "folder-1",
    subject: "Status update",
    preview: "Preview text",
    from: [{ name: "Alice", email: "alice@example.com" }],
    sentAt: "2026-10-03T00:00:00.000Z",
    isRead,
    isStarred: false,
  };
}

afterEach(cleanup);

describe("MailMessageRow", () => {
  it("shows an explicit unread marker and stronger unread styling", () => {
    render(<MailMessageRow message={message(false)} selected={false} onSelect={vi.fn()} />);

    const row = screen.getByTestId("mail-message-message-1");
    const indicator = screen.getByTestId("mail-unread-indicator");

    expect(row.getAttribute("data-read-state")).toBe("unread");
    expect(row.getAttribute("aria-label")).toBe("未读邮件：Status update");
    expect(indicator.classList.contains("bg-primary")).toBe(true);
    expect(row.className).toContain("bg-primary/");
  });

  it("removes the unread marker once the message is read", () => {
    render(<MailMessageRow message={message(true)} selected={false} onSelect={vi.fn()} />);

    expect(screen.getByTestId("mail-message-message-1").getAttribute("data-read-state")).toBe("read");
    expect(screen.getByTestId("mail-unread-indicator").classList.contains("bg-transparent")).toBe(true);
  });
});
