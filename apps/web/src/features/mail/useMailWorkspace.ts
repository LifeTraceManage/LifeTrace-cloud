import { useCallback, useEffect, useMemo, useState } from "react";
import { useApp } from "../../app/AppContext";
import { MailApi } from "./api";
import type {
  ComposeMailInput,
  MailAccount,
  MailAccountInput,
  MailConnectionTest,
  MailDraft,
  MailDraftAttachment,
  MailDraftInput,
  MailIdentity,
  MailIdentityInput,
  MailMessageDetail,
  MailMessageSummary,
  MailRuntimeInfo,
  Mailbox,
} from "./types";

export function useMailWorkspace(
  mailboxRole: string,
  query: string,
  accountId: string | null,
) {
  const { session } = useApp();
  const api = useMemo(() => new MailApi(session?.csrfToken), [session?.csrfToken]);
  const [runtime, setRuntime] = useState<MailRuntimeInfo>({
    status: "unavailable",
    provider: "lifetrace-cloud",
    lastCheckedAt: new Date(0).toISOString(),
  });
  const [accounts, setAccounts] = useState<MailAccount[]>([]);
  const [identities, setIdentities] = useState<MailIdentity[]>([]);
  const [mailboxes, setMailboxes] = useState<Mailbox[]>([]);
  const [drafts, setDrafts] = useState<MailDraft[]>([]);
  const [messages, setMessages] = useState<MailMessageSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedMessage, setSelectedMessage] = useState<MailMessageDetail | null>(null);
  const [runtimeLoading, setRuntimeLoading] = useState(true);
  const [listLoading, setListLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [error, setError] = useState("");

  const loadBootstrap = useCallback(async () => {
    setRuntimeLoading(true);
    setError("");
    try {
      const [nextAccounts, nextIdentities, nextDrafts] = await Promise.all([
        api.accounts(),
        api.identities(),
        api.drafts(),
      ]);
      const folderGroups = await Promise.all(nextAccounts.map((item) => api.mailboxes(item.id)));
      setRuntime({
        status: "ready",
        provider: "lifetrace-cloud",
        lastCheckedAt: new Date().toISOString(),
      });
      setAccounts(nextAccounts);
      setIdentities(nextIdentities);
      setMailboxes(folderGroups.flat());
      setDrafts(nextDrafts);
    } catch (cause) {
      setRuntime({
        status: "unavailable",
        provider: "lifetrace-cloud",
        lastCheckedAt: new Date().toISOString(),
      });
      setAccounts([]);
      setIdentities([]);
      setMailboxes([]);
      setDrafts([]);
      setMessages([]);
      setError(cause instanceof Error ? cause.message : "LifeTrace Mail 暂不可用");
    } finally {
      setRuntimeLoading(false);
    }
  }, [api]);

  const loadMessages = useCallback(async (silent = false) => {
    if (runtime.status !== "ready") return;
    if (!silent) setListLoading(true);
    setError("");
    try {
      const page = await api.messages({
        accountId,
        mailboxRole,
        query: query.trim(),
        starredOnly: mailboxRole === "starred",
        limit: 100,
      });
      setMessages(page.items);
      setSelectedId((current) =>
        current && page.items.some((item) => item.id === current)
          ? current
          : page.items[0]?.id ?? null
      );
    } catch (cause) {
      setMessages([]);
      setSelectedId(null);
      setError(cause instanceof Error ? cause.message : "无法读取邮件列表");
    } finally {
      if (!silent) setListLoading(false);
    }
  }, [accountId, api, mailboxRole, query, runtime.status]);

  useEffect(() => {
    void loadBootstrap();
  }, [loadBootstrap]);

  useEffect(() => {
    void loadMessages();
  }, [loadMessages]);

  useEffect(() => {
    if (runtime.status !== "ready" || typeof WebSocket === "undefined") return;

    let socket: WebSocket | null = null;
    let reconnectTimer: number | undefined;
    let refreshTimer: number | undefined;
    let disposed = false;

    const scheduleRefresh = () => {
      if (refreshTimer !== undefined) window.clearTimeout(refreshTimer);
      refreshTimer = window.setTimeout(() => {
        void loadMessages(true);
      }, 200);
    };

    const connect = () => {
      if (disposed) return;
      socket = new WebSocket(api.eventsUrl());
      socket.onmessage = (event) => {
        try {
          const payload = JSON.parse(event.data) as { type?: string };
          if (payload.type === "mail.updated") scheduleRefresh();
        } catch {
          scheduleRefresh();
        }
      };
      socket.onclose = () => {
        if (!disposed) reconnectTimer = window.setTimeout(connect, 1500);
      };
    };

    const refreshOnFocus = () => {
      if (document.visibilityState === "visible") void loadMessages(true);
    };

    connect();
    window.addEventListener("focus", refreshOnFocus);
    document.addEventListener("visibilitychange", refreshOnFocus);

    return () => {
      disposed = true;
      if (refreshTimer !== undefined) window.clearTimeout(refreshTimer);
      if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
      socket?.close();
      window.removeEventListener("focus", refreshOnFocus);
      document.removeEventListener("visibilitychange", refreshOnFocus);
    };
  }, [api, loadMessages, runtime.status]);

  useEffect(() => {
    let active = true;
    if (!selectedId || runtime.status !== "ready") {
      setSelectedMessage(null);
      return;
    }
    setDetailLoading(true);
    api.message(selectedId)
      .then((message) => { if (active) setSelectedMessage(message); })
      .catch((cause) => { if (active) setError(cause instanceof Error ? cause.message : "无法读取邮件详情"); })
      .finally(() => { if (active) setDetailLoading(false); });
    return () => { active = false; };
  }, [api, runtime.status, selectedId]);

  const refresh = useCallback(async () => {
    await loadBootstrap();
    await loadMessages();
  }, [loadBootstrap, loadMessages]);

  const send = useCallback(async (input: ComposeMailInput) => {
    await api.send(input);
    await loadBootstrap();
    await loadMessages();
  }, [api, loadBootstrap, loadMessages]);

  const markRead = useCallback(async (messageId: string, isRead: boolean) => {
    setError("");
    setMessages((items) => items.map((item) => item.id === messageId ? { ...item, isRead } : item));
    setSelectedMessage((item) => item?.id === messageId ? { ...item, isRead } : item);
    try {
      await api.markRead(messageId, isRead);
    } catch (cause) {
      setMessages((items) => items.map((item) => item.id === messageId ? { ...item, isRead: !isRead } : item));
      setSelectedMessage((item) => item?.id === messageId ? { ...item, isRead: !isRead } : item);
      setError(cause instanceof Error ? cause.message : "更新邮件已读状态失败");
    }
  }, [api]);

  const setStarred = useCallback(async (messageId: string, isStarred: boolean) => {
    setError("");
    setMessages((items) => items.map((item) => item.id === messageId ? { ...item, isStarred } : item));
    setSelectedMessage((item) => item?.id === messageId ? { ...item, isStarred } : item);
    try {
      await api.setStarred(messageId, isStarred);
    } catch (cause) {
      setMessages((items) => items.map((item) => item.id === messageId ? { ...item, isStarred: !isStarred } : item));
      setSelectedMessage((item) => item?.id === messageId ? { ...item, isStarred: !isStarred } : item);
      setError(cause instanceof Error ? cause.message : "更新邮件星标失败");
    }
  }, [api]);

  const move = useCallback(async (
    messageId: string,
    destination: "archive" | "trash" | "inbox",
  ) => {
    setError("");
    const previousMessages = messages;
    const previousSelectedId = selectedId;
    const previousSelectedMessage = selectedMessage;
    const removedIndex = previousMessages.findIndex((item) => item.id === messageId);
    const nextMessages = previousMessages.filter((item) => item.id !== messageId);
    const nextSelectedId = previousSelectedId === messageId
      ? nextMessages[Math.min(Math.max(removedIndex, 0), Math.max(nextMessages.length - 1, 0))]?.id ?? null
      : previousSelectedId;

    // Optimistic update: the UI reacts immediately while the IMAP MOVE happens
    // in the background. The previous state is restored if the server rejects it.
    setMessages(nextMessages);
    setSelectedId(nextSelectedId);
    if (previousSelectedId === messageId) setSelectedMessage(null);

    try {
      await api.move(messageId, destination);
    } catch (cause) {
      setMessages(previousMessages);
      setSelectedId(previousSelectedId);
      setSelectedMessage(previousSelectedMessage);
      setError(cause instanceof Error ? cause.message : "移动邮件失败");
    }
  }, [api, messages, selectedId, selectedMessage]);

  const connectAccount = useCallback(async (input: MailAccountInput) => {
    const created = await api.createAccount(input);
    await loadBootstrap();
    return created;
  }, [api, loadBootstrap]);

  const disconnectAccount = useCallback(async (id: string) => {
    await api.disconnectAccount(id);
    await loadBootstrap();
    await loadMessages();
  }, [api, loadBootstrap, loadMessages]);

  const testAccount = useCallback(
    (id: string): Promise<MailConnectionTest> => api.testAccount(id),
    [api],
  );

  const syncAccount = useCallback(async (id: string) => {
    const result = await api.syncAccount(id);
    await loadBootstrap();
    await loadMessages();
    return result;
  }, [api, loadBootstrap, loadMessages]);

  const createIdentity = useCallback(async (input: MailIdentityInput) => {
    const value = await api.createIdentity(input);
    await loadBootstrap();
    return value;
  }, [api, loadBootstrap]);

  const updateIdentity = useCallback(async (id: string, input: MailIdentityInput) => {
    const value = await api.updateIdentity(id, input);
    await loadBootstrap();
    return value;
  }, [api, loadBootstrap]);

  const deleteIdentity = useCallback(async (id: string) => {
    await api.deleteIdentity(id);
    await loadBootstrap();
  }, [api, loadBootstrap]);

  const saveDraft = useCallback(async (input: MailDraftInput, id?: string | null) => {
    const value = id ? await api.updateDraft(id, input) : await api.createDraft(input);
    await loadBootstrap();
    return value;
  }, [api, loadBootstrap]);

  const deleteDraft = useCallback(async (id: string) => {
    await api.deleteDraft(id);
    await loadBootstrap();
  }, [api, loadBootstrap]);

  const sendDraft = useCallback(async (id: string) => {
    const result = await api.sendDraft(id);
    await loadBootstrap();
    await loadMessages();
    return result;
  }, [api, loadBootstrap, loadMessages]);


  const listDraftAttachments = useCallback(
    (id: string): Promise<MailDraftAttachment[]> => api.draftAttachments(id),
    [api],
  );

  const uploadDraftAttachment = useCallback(
    async (id: string, file: File): Promise<MailDraftAttachment> => api.uploadDraftAttachment(id, file),
    [api],
  );

  const deleteDraftAttachment = useCallback(
    async (draftId: string, attachmentId: string): Promise<void> => {
      await api.deleteDraftAttachment(draftId, attachmentId);
    },
    [api],
  );

  return {
    runtime, accounts, identities, mailboxes, drafts, messages, selectedId, selectedMessage,
    runtimeLoading, listLoading, detailLoading, error,
    setSelectedId, refresh, send, markRead, setStarred, move,
    connectAccount, disconnectAccount, testAccount, syncAccount,
    createIdentity, updateIdentity, deleteIdentity,
    saveDraft, deleteDraft, sendDraft,
    listDraftAttachments, uploadDraftAttachment, deleteDraftAttachment,
  };
}
