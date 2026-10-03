// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  accounts: vi.fn(),
  identities: vi.fn(),
  drafts: vi.fn(),
  mailboxes: vi.fn(),
  messages: vi.fn(),
  message: vi.fn(),
  markRead: vi.fn(),
  markReadBulk: vi.fn(),
  eventsUrl: vi.fn(() => "ws://127.0.0.1/mail"),
}));

vi.mock("../../app/AppContext", () => ({
  useApp: () => ({
    session: {
      csrfToken: "csrf",
      user: { id: "user-1" },
    },
  }),
}));

vi.mock("./api", () => ({
  MailApi: class {
    accounts = apiMock.accounts;
    identities = apiMock.identities;
    drafts = apiMock.drafts;
    mailboxes = apiMock.mailboxes;
    messages = apiMock.messages;
    message = apiMock.message;
    markRead = apiMock.markRead;
    markReadBulk = apiMock.markReadBulk;
    eventsUrl = apiMock.eventsUrl;
  },
}));

import { useMailWorkspace } from "./useMailWorkspace";

function Harness() {
  const workspace = useMailWorkspace("inbox", "", null);
  return <div>
    <span data-testid="selected-read-state">
      {workspace.selectedMessage ? (workspace.selectedMessage.isRead ? "read" : "unread") : "none"}
    </span>
    <span data-testid="unread-count">{workspace.messages.filter((message) => !message.isRead).length}</span>
    <button type="button" onClick={() => workspace.openMessage("message-1")}>open</button>
    <button type="button" onClick={() => void workspace.markAllRead(workspace.messages.filter((message) => !message.isRead).map((message) => message.id))}>bulk</button>
  </div>;
}

describe("useMailWorkspace read state", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal("WebSocket", undefined);

    apiMock.accounts.mockResolvedValue([]);
    apiMock.identities.mockResolvedValue([]);
    apiMock.drafts.mockResolvedValue([]);
    apiMock.messages.mockResolvedValue({
      items: [{
        id: "message-1",
        accountId: "account-1",
        mailboxId: "folder-1",
        subject: "Unread mail",
        preview: "Preview",
        from: [{ email: "alice@example.com" }],
        sentAt: "2026-10-03T00:00:00.000Z",
        isRead: false,
        isStarred: false,
      }],
      hasMore: false,
      nextOffset: 1,
    });
    apiMock.message.mockResolvedValue({
      id: "message-1",
      accountId: "account-1",
      mailboxId: "folder-1",
      subject: "Unread mail",
      preview: "Preview",
      from: [{ email: "alice@example.com" }],
      to: [{ email: "user@example.com" }],
      sentAt: "2026-10-03T00:00:00.000Z",
      isRead: false,
      isStarred: false,
      attachments: [],
      text: "Body",
      html: null,
    });
    apiMock.markRead.mockResolvedValue(undefined);
    apiMock.markReadBulk.mockResolvedValue({
      updatedCount: 0,
      updatedMessageIds: [],
      failedMessageIds: [],
    });
  });

  it("does not consume unread state on automatic selection, then marks read when explicitly opened", async () => {
    render(<Harness />);

    await waitFor(() => expect(screen.getByTestId("selected-read-state").textContent).toBe("unread"));
    expect(apiMock.markRead).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "open" }));

    await waitFor(() => expect(apiMock.markRead).toHaveBeenCalledWith("message-1", true));
    await waitFor(() => expect(screen.getByTestId("selected-read-state").textContent).toBe("read"));
  });

  it("marks unread messages in one bulk call and restores only failures", async () => {
    apiMock.messages.mockResolvedValue({
      items: [
        {
          id: "message-1",
          accountId: "account-1",
          mailboxId: "folder-1",
          subject: "First unread",
          preview: "Preview",
          from: [{ email: "alice@example.com" }],
          sentAt: "2026-10-03T00:00:00.000Z",
          isRead: false,
          isStarred: false,
        },
        {
          id: "message-2",
          accountId: "account-1",
          mailboxId: "folder-1",
          subject: "Second unread",
          preview: "Preview",
          from: [{ email: "bob@example.com" }],
          sentAt: "2026-10-03T00:01:00.000Z",
          isRead: false,
          isStarred: false,
        },
      ],
      hasMore: false,
      nextOffset: 2,
    });
    apiMock.markReadBulk.mockResolvedValue({
      updatedCount: 1,
      updatedMessageIds: ["message-1"],
      failedMessageIds: ["message-2"],
    });

    render(<Harness />);
    await waitFor(() => expect(screen.getByTestId("unread-count").textContent).toBe("2"));

    fireEvent.click(screen.getByRole("button", { name: "bulk" }));

    await waitFor(() => expect(apiMock.markReadBulk).toHaveBeenCalledTimes(1));
    expect(apiMock.markReadBulk).toHaveBeenCalledWith(["message-1", "message-2"], true);
    await waitFor(() => expect(screen.getByTestId("unread-count").textContent).toBe("1"));
  });
});
