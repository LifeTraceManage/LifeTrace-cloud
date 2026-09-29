import { useMemo, useState, type FormEvent } from "react";
import { Bot, Send } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, PageHeader, Textarea } from "../../components/ui";
import { AssistantApi } from "../../services/core";

type Message = { role: "user" | "assistant"; content: string; provider?: string };

export function AssistantPage() {
  const { session } = useApp();
  const api = useMemo(() => new AssistantApi(), []);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState("");

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
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "AI 请求失败");
    } finally {
      setAsking(false);
    }
  }

  return (
    <div className="page-shell">
      <PageHeader title="AI 助手" />
      <div className="mx-auto max-w-4xl">
        <Card>
          <CardContent className="pt-5">
            <div className="min-h-[420px] space-y-4">
              {!messages.length ? (
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
              <Button type="submit" disabled={asking || !prompt.trim()}>
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
