import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type PropsWithChildren,
} from "react";
import { useApp } from "../../app/AppContext";
import {
  AssistantApi,
  type AssistantApproval,
  type AssistantSession,
} from "../../services/core";

export interface AgentSelectedEntityContext {
  entityType: string;
  entityId: string;
}

export interface AgentTemporalContext {
  date?: string;
  rangeStart?: string;
  rangeEnd?: string;
}

export interface AgentSearchContext {
  query?: string;
  folderId?: string;
  projectId?: string;
  mailbox?: string;
}

export interface AgentPageContext {
  workspace: string;
  view?: string;
  label?: string;
  selectedEntity?: AgentSelectedEntityContext;
  temporalContext?: AgentTemporalContext;
  searchContext?: AgentSearchContext;
}

export type AgentSidebarMessage = {
  role: "user" | "assistant";
  content: string;
  provider?: string;
};

interface AgentSidebarContextValue {
  open: boolean;
  historyOpen: boolean;
  draft: string;
  sessionId: string | null;
  sessions: AssistantSession[];
  messages: AgentSidebarMessage[];
  approvals: AssistantApproval[];
  asking: boolean;
  loadingHistory: boolean;
  decidingApprovalId: string | null;
  deletingSessionId: string | null;
  error: string;
  pageContext: AgentPageContext | null;
  setOpen(open: boolean): void;
  toggle(): void;
  setHistoryOpen(open: boolean): void;
  setDraft(value: string): void;
  setRouteContext(context: AgentPageContext | null): void;
  setPageContext(context: AgentPageContext | null): void;
  send(): Promise<void>;
  openSession(id: string): Promise<void>;
  newConversation(): void;
  deleteConversation(id: string): Promise<void>;
  decideApproval(id: string, decision: "approve" | "reject"): Promise<void>;
}

const AgentSidebarContext = createContext<AgentSidebarContextValue | null>(null);
const OPEN_KEY = "lifetrace:agent-sidebar:open";

function initialOpen(): boolean {
  if (typeof window === "undefined") return false;
  return localStorage.getItem(OPEN_KEY) === "true";
}

export function AgentSidebarProvider({ children }: PropsWithChildren) {
  const { session } = useApp();
  const api = useMemo(() => new AssistantApi(), []);
  const [open, setOpenState] = useState(initialOpen);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [draft, setDraft] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessions, setSessions] = useState<AssistantSession[]>([]);
  const [messages, setMessages] = useState<AgentSidebarMessage[]>([]);
  const [approvals, setApprovals] = useState<AssistantApproval[]>([]);
  const [asking, setAsking] = useState(false);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [decidingApprovalId, setDecidingApprovalId] = useState<string | null>(null);
  const [deletingSessionId, setDeletingSessionId] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [routeContext, setRouteContext] = useState<AgentPageContext | null>(null);
  const [pageContext, setPageContext] = useState<AgentPageContext | null>(null);
  const effectivePageContext = pageContext ?? routeContext;

  const setOpen = useCallback((value: boolean) => {
    setOpenState(value);
    if (typeof window !== "undefined") {
      localStorage.setItem(OPEN_KEY, value ? "true" : "false");
    }
  }, []);

  const toggle = useCallback(() => setOpen(!open), [open, setOpen]);

  const refreshSessions = useCallback(async () => {
    if (!session) {
      setSessions([]);
      return;
    }
    try {
      setSessions(await api.listSessions());
    } catch {
      // Conversation remains usable if history refresh fails.
    }
  }, [api, session]);

  useEffect(() => {
    if (!session) {
      setSessionId(null);
      setSessions([]);
      setMessages([]);
      setApprovals([]);
      return;
    }
    void refreshSessions();
  }, [refreshSessions, session]);

  const openSession = useCallback(async (id: string) => {
    if (!session || asking || loadingHistory) return;
    setLoadingHistory(true);
    setError("");
    try {
      const [items, approvalItems] = await Promise.all([
        api.listMessages(id),
        api.listApprovals(id),
      ]);
      setSessionId(id);
      setApprovals(approvalItems);
      setMessages(
        items
          .filter((item) => item.role === "user" || item.role === "assistant")
          .map((item) => ({
            role: item.role as "user" | "assistant",
            content: item.content,
            provider: item.role === "assistant" ? item.provider ?? undefined : undefined,
          })),
      );
      setHistoryOpen(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "会话加载失败");
    } finally {
      setLoadingHistory(false);
    }
  }, [api, asking, loadingHistory, session]);

  const newConversation = useCallback(() => {
    if (asking) return;
    setSessionId(null);
    setMessages([]);
    setApprovals([]);
    setDraft("");
    setError("");
    setHistoryOpen(false);
  }, [asking]);

  const deleteConversation = useCallback(async (id: string) => {
    if (!session || deletingSessionId || asking) return;
    const target = sessions.find((item) => item.id === id);
    if (typeof window !== "undefined" && !window.confirm(
      `删除会话“${target?.title ?? "未命名会话"}”？该会话的消息和审批记录也会一起删除。`,
    )) return;
    setDeletingSessionId(id);
    setError("");
    try {
      await api.deleteSession(id, session.csrfToken);
      setSessions((items) => items.filter((item) => item.id !== id));
      if (sessionId === id) newConversation();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "会话删除失败");
    } finally {
      setDeletingSessionId(null);
    }
  }, [api, asking, deletingSessionId, newConversation, session, sessionId, sessions]);

  const send = useCallback(async () => {
    const text = draft.trim();
    if (!session || !text || asking) return;
    setDraft("");
    setMessages((value) => [...value, { role: "user", content: text }]);
    setAsking(true);
    setError("");
    try {
      const reply = await api.ask(text, session.csrfToken, sessionId, effectivePageContext);
      setSessionId(reply.sessionId);
      setMessages((value) => [
        ...value,
        {
          role: "assistant",
          content: reply.reply,
          provider: reply.model ? `${reply.provider} · ${reply.model}` : reply.provider,
        },
      ]);
      setApprovals(await api.listApprovals(reply.sessionId));
      await refreshSessions();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "AI 请求失败");
    } finally {
      setAsking(false);
    }
  }, [api, asking, draft, effectivePageContext, refreshSessions, session, sessionId]);

  const decideApproval = useCallback(async (
    approvalId: string,
    decision: "approve" | "reject",
  ) => {
    if (!session || decidingApprovalId) return;
    setDecidingApprovalId(approvalId);
    setError("");
    try {
      const result = await api.decideApproval(approvalId, decision, session.csrfToken);
      setApprovals((items) =>
        items.map((item) => (item.id === approvalId ? result.approval : item)),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "审批操作失败");
      if (sessionId) {
        try {
          setApprovals(await api.listApprovals(sessionId));
        } catch {
          // Keep current cards if refresh also fails.
        }
      }
    } finally {
      setDecidingApprovalId(null);
    }
  }, [api, decidingApprovalId, session, sessionId]);

  const value = useMemo<AgentSidebarContextValue>(() => ({
    open,
    historyOpen,
    draft,
    sessionId,
    sessions,
    messages,
    approvals,
    asking,
    loadingHistory,
    decidingApprovalId,
    deletingSessionId,
    error,
    pageContext: effectivePageContext,
    setOpen,
    toggle,
    setHistoryOpen,
    setDraft,
    setRouteContext,
    setPageContext,
    send,
    openSession,
    newConversation,
    deleteConversation,
    decideApproval,
  }), [
    open, historyOpen, draft, sessionId, sessions, messages, approvals, asking,
    loadingHistory, decidingApprovalId, deletingSessionId, error, effectivePageContext,
    setOpen, toggle, send, openSession, newConversation, deleteConversation, decideApproval,
  ]);

  return <AgentSidebarContext.Provider value={value}>{children}</AgentSidebarContext.Provider>;
}

export function useAgentSidebar(): AgentSidebarContextValue {
  const value = useContext(AgentSidebarContext);
  if (!value) throw new Error("useAgentSidebar must be used inside AgentSidebarProvider");
  return value;
}

export function useAgentPageContext(context: AgentPageContext | null): void {
  const { setPageContext } = useAgentSidebar();
  const serialized = JSON.stringify(context);
  useEffect(() => {
    setPageContext(context);
    return () => setPageContext(null);
  }, [serialized, setPageContext]);
}

export function useAgentRouteContext(context: AgentPageContext | null): void {
  const { setRouteContext } = useAgentSidebar();
  const serialized = JSON.stringify(context);
  useEffect(() => {
    setRouteContext(context);
    return () => setRouteContext(null);
  }, [serialized, setRouteContext]);
}

export function agentContextFromPath(pathname: string): AgentPageContext {
  if (pathname.startsWith("/execute/")) {
    const view = pathname.split("/")[2] || "today";
    return { workspace: "execution", view, label: `Execute · ${view}` };
  }
  if (pathname.startsWith("/mail")) return { workspace: "mail", view: "mail", label: "Mail" };
  if (pathname.startsWith("/notes")) return { workspace: "notes", view: "notes", label: "Notes" };
  if (pathname.startsWith("/finance")) return { workspace: "finance", view: "finance", label: "Finance" };
  if (pathname.startsWith("/app/fitness")) return { workspace: "fitness", view: "fitness", label: "Fitness" };
  if (pathname.startsWith("/app/health")) return { workspace: "health", view: "health", label: "Health" };
  if (pathname.startsWith("/app/settings")) return { workspace: "settings", view: "settings", label: "Settings" };
  return { workspace: "lifetrace", view: "workspace", label: "LifeTrace" };
}
