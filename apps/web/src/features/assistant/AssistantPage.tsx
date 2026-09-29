import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Bot, Plus, Send } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, PageHeader, Textarea } from "../../components/ui";
import { AssistantApi, type AssistantSession } from "../../services/core";

type Message = { role: "user" | "assistant"; content: string; provider?: string };

export function AssistantPage() {
  const { session } = useApp();
  const api = useMemo(() => new AssistantApi(), []);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessions, setSessions] = useState<AssistantSession[]>([]);
  const [prompt, setPrompt] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [asking, setAsking] = useState(false);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!session) return;
    let active = true;
    void api
      .listSessions()
      .then((items) => {
        if (active) setSessions(items);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : "会话加载失败");
      });
    return () => {
      active = false;
    };
  }, [api, session]);

  async function refreshSessions() {
    if (!session) return;
    try {
      setSessions(await api.listSessions());
    } catch {
      // Conversation itself is still usable if history refresh fails.
    }
  }

  async function openSession(id: string) {
    if (asking || loadingHistory) return;
    setLoadingHistory(true);
    setError("");
    try {
      const items = await api.listMessages(id);
      setSessionId(id);
      setMessages(
        items
          .filter((item) => item.role === "user" || item.role === "assistant")
          .map((item) => ({
            role: item.role as "user" | "assistant",
            content: item.content,
            provider: item.role === "assistant" ? item.provider ?? undefined : undefined,
          })),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "会话加载失败");
    } finally {
      setLoadingHistory(false);
    }
  }

  function newConversation() {
    if (asking) return;
    setSessionId(null);
    setMessages([]);
    setPrompt("");
    setError("");
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!session || !prompt.trim()) return;
    const text = prompt.trim();
    setPrompt("");
    setMessages((value) => [...value, { role: "user", content: text }]);
    setAsking(true);
    setError("");
    try {
      const reply = await api.ask(text, session.csrfToken, sessionId);
      setSessionId(reply.sessionId);
      setMessages((value) => [
        ...value,
        {
          role: "assistant",
          content: reply.reply,
          provider: reply.model ? `${reply.provider} · ${reply.model}` : reply.provider,
        },
      ]);
      await refreshSessions();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "AI 请求失败");
    } finally {
      setAsking(false);
    }
  }

  return (
    <div className="page-shell">
      <PageHeader title="AI 助手" />
      <div className="mx-auto grid max-w-6xl gap-4 lg:grid-cols-[260px_minmax(0,1fr)]">
        <Card className="h-fit">
          <CardContent className="space-y-3 pt-5">
            <Button className="w-full justify-start" variant="outline" onClick={newConversation} disabled={asking}>
              <Plus size={16} />
              新对话
            </Button>
            <div className="space-y-1">
              {sessions.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => void openSession(item.id)}
                  disabled={asking || loadingHistory}
                  className={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-muted ${
                    item.id === sessionId ? "bg-muted font-medium" : ""
                  }`}
                >
                  <div className="truncate">{item.title}</div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {new Date(item.lastMessageAt ?? item.updatedAt).toLocaleString()}
                  </div>
                </button>
              ))}
              {!sessions.length ? (
                <div className="px-2 py-3 text-xs text-muted-foreground">暂无历史会话</div>
              ) : null}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-5">
            <div className="min-h-[520px] space-y-4">
              {loadingHistory ? (
                <div className="text-sm text-muted-foreground">正在加载会话…</div>
              ) : !messages.length ? (
                <EmptyState
                  icon={<Bot size={26} />}
                  title="问问 LifeTrace"
                  description="可以直接询问任务、日程、笔记、邮件等已同步数据；第一阶段仅开放只读分析。"
                />
              ) : (
                messages.map((message, index) => (
                  <div
                    key={index}
                    className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
                  >
                    <div
                      className={`max-w-[85%] rounded-lg px-4 py-3 text-sm leading-6 ${
                        message.role === "user" ? "bg-primary text-primary-foreground" : "bg-muted"
                      }`}
                    >
                      <div className="whitespace-pre-wrap">{message.content}</div>
                      {message.provider ? <Badge className="mt-2">{message.provider}</Badge> : null}
                    </div>
                  </div>
                ))
              )}
              {asking ? <div className="text-sm text-muted-foreground">正在查询并分析云端记录…</div> : null}
              {error ? <div className="text-sm text-destructive">{error}</div> : null}
            </div>
            <form className="mt-4 flex items-end gap-2 border-t pt-4" onSubmit={(event) => void submit(event)}>
              <Textarea
                className="min-h-20 flex-1"
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
                placeholder="输入问题…"
              />
              <Button type="submit" disabled={asking || loadingHistory || !prompt.trim()}>
                <Send size={16} />
                发送
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
