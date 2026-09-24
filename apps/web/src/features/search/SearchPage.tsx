import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Search } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Card, CardContent, EmptyState, Input, PageHeader } from "../../components/ui";
import { searchEntities, type SearchHit } from "../../services/core";

const routeMap: Record<string, string> = {
  "/execution": "/app/execution",
  "/execution/goals": "/app/execution",
  "/habits": "/app/habits",
  "/fitness": "/app/fitness",
  "/finance/transactions": "/app/finance/transactions",
  "/finance/accounts": "/app/finance/accounts",
  "/notes": "/notes",
  "/english/articles": "/app/english/articles",
  "/english/vocabulary": "/app/english/vocabulary",
  "/english/stats": "/app/english/stats",
  "/review": "/app/review",
};

function destination(hit: SearchHit): string {
  if (hit.entityType === "note.note") {
    return `/notes?note=${encodeURIComponent(hit.id)}`;
  }
  return routeMap[hit.route] ?? "/app/today";
}

export function SearchPage() {
  const { state } = useApp();
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const hits = useMemo(() => searchEntities(state, query), [state, query]);

  return <div className="page-shell">
    <PageHeader title="全局搜索" />
    <div className="mx-auto max-w-3xl">
      <Card>
        <CardContent className="pt-5">
          <div className="flex items-center gap-2">
            <Search size={18} className="text-muted-foreground" />
            <Input autoFocus className="border-0 bg-muted/50 focus:ring-0" placeholder="输入关键词…" value={query} onChange={(event) => setQuery(event.target.value)} />
          </div>
          <div className="mt-4">
            {query && !hits.length ? <EmptyState title="没有结果" /> : hits.map((hit) => <button
              key={`${hit.entityType}-${hit.id}`}
              onClick={() => navigate(destination(hit))}
              className="mb-1 flex w-full items-start gap-3 rounded-md px-3 py-3 text-left hover:bg-muted"
            >
              <Badge>{hit.entityType}</Badge>
              <div className="min-w-0">
                <div className="truncate text-sm font-medium">{hit.title}</div>
                <div className="mt-1 line-clamp-2 text-xs text-muted-foreground">{hit.subtitle}</div>
              </div>
            </button>)}
          </div>
        </CardContent>
      </Card>
    </div>
  </div>;
}
