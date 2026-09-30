import { API_BASE } from "./base";
import { browserFetch } from "./http";
import type { FetchLike } from "./types";

export interface AssistantReply {
  reply: string;
  provider: string;
  sessionId: string;
  runId: string;
  model?: string | null;
}

export interface AssistantSession {
  id: string;
  title: string;
  status: "active" | "archived";
  createdAt: string;
  updatedAt: string;
  lastMessageAt?: string | null;
}

export interface AssistantMessage {
  id: string;
  sessionId: string;
  runId?: string | null;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  provider?: string | null;
  metadataJson: Record<string, unknown>;
  createdAt: string;
}

export interface AssistantApproval {
  id: string;
  runId: string;
  sessionId: string;
  toolCallId?: string | null;
  actionName: string;
  actionJson: Record<string, unknown>;
  status: "pending" | "approved" | "rejected" | "expired" | "cancelled" | string;
  requestedAt: string;
  decidedAt?: string | null;
  expiresAt?: string | null;
  supersededByApprovalId?: string | null;
  cancellationReason?: string | null;
}

export interface ApprovalDecisionResult {
  approval: AssistantApproval;
  result?: unknown;
}

export class AssistantApi {
  constructor(private readonly fetcher: FetchLike = browserFetch) {}

  async ask(prompt: string, csrfToken: string, sessionId?: string | null): Promise<AssistantReply> {
    const response = await this.fetcher(`${API_BASE}/api/v1/web/assistant`, {
      method: "POST",
      credentials: "include",
      headers: { "content-type": "application/json", "x-csrf-token": csrfToken },
      body: JSON.stringify({
        prompt: prompt.trim(),
        ...(sessionId ? { sessionId } : {}),
      }),
    });
    const payload = await response.json() as Partial<AssistantReply> & {
      message?: string;
      error?: { message?: string };
    };
    if (!response.ok) {
      throw new Error(payload.message || payload.error?.message || `AI 请求失败 (${response.status})`);
    }
    if (!payload.reply || !payload.sessionId || !payload.runId) {
      throw new Error("AI 服务返回的数据不完整");
    }
    return {
      reply: payload.reply,
      provider: payload.provider ?? "local",
      sessionId: payload.sessionId,
      runId: payload.runId,
      model: payload.model ?? null,
    };
  }

  async listSessions(limit = 30): Promise<AssistantSession[]> {
    const response = await this.fetcher(
      `${API_BASE}/api/v1/assistant/sessions?limit=${encodeURIComponent(String(limit))}`,
      { credentials: "include" },
    );
    const payload = await response.json() as {
      items?: AssistantSession[];
      message?: string;
      error?: { message?: string };
    };
    if (!response.ok) {
      throw new Error(payload.message || payload.error?.message || `会话加载失败 (${response.status})`);
    }
    return payload.items ?? [];
  }

  async deleteSession(sessionId: string, csrfToken: string): Promise<void> {
    const response = await this.fetcher(
      `${API_BASE}/api/v1/assistant/sessions/${encodeURIComponent(sessionId)}`,
      {
        method: "DELETE",
        credentials: "include",
        headers: { "x-csrf-token": csrfToken },
      },
    );
    if (!response.ok) {
      let message = `会话删除失败 (${response.status})`;
      try {
        const payload = await response.json() as {
          message?: string;
          error?: { message?: string };
        };
        message = payload.message || payload.error?.message || message;
      } catch {
        // Keep the status-based fallback.
      }
      throw new Error(message);
    }
  }

  async listMessages(sessionId: string, limit = 100): Promise<AssistantMessage[]> {
    const response = await this.fetcher(
      `${API_BASE}/api/v1/assistant/sessions/${encodeURIComponent(sessionId)}/messages?limit=${encodeURIComponent(String(limit))}`,
      { credentials: "include" },
    );
    const payload = await response.json() as {
      items?: AssistantMessage[];
      message?: string;
      error?: { message?: string };
    };
    if (!response.ok) {
      throw new Error(payload.message || payload.error?.message || `消息加载失败 (${response.status})`);
    }
    return payload.items ?? [];
  }

  async listApprovals(sessionId: string, limit = 20): Promise<AssistantApproval[]> {
    const response = await this.fetcher(
      `${API_BASE}/api/v1/assistant/sessions/${encodeURIComponent(sessionId)}/approvals?limit=${encodeURIComponent(String(limit))}`,
      { credentials: "include" },
    );
    const payload = await response.json() as {
      items?: AssistantApproval[];
      message?: string;
      error?: { message?: string };
    };
    if (!response.ok) {
      throw new Error(payload.message || payload.error?.message || `审批加载失败 (${response.status})`);
    }
    return payload.items ?? [];
  }

  async decideApproval(
    approvalId: string,
    decision: "approve" | "reject",
    csrfToken: string,
  ): Promise<ApprovalDecisionResult> {
    const response = await this.fetcher(
      `${API_BASE}/api/v1/assistant/approvals/${encodeURIComponent(approvalId)}/decision`,
      {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json", "x-csrf-token": csrfToken },
        body: JSON.stringify({ decision }),
      },
    );
    const payload = await response.json() as Partial<ApprovalDecisionResult> & {
      message?: string;
      error?: { message?: string };
    };
    if (!response.ok) {
      throw new Error(payload.message || payload.error?.message || `审批操作失败 (${response.status})`);
    }
    if (!payload.approval) {
      throw new Error("审批服务返回的数据不完整");
    }
    return { approval: payload.approval, result: payload.result };
  }
}
