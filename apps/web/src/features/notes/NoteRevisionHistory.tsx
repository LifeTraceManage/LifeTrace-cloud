import { History, RotateCcw } from "lucide-react";
import { Badge, Button, Dialog, EmptyState } from "../../components/ui";
import { number, text } from "../../lib/entities";
import type { JsonEntity } from "../../services/core";
import { markdownSummary } from "./markdown";

export function NoteRevisionHistory({
  open,
  onOpenChange,
  noteId,
  revisions,
  onRestore,
}: {
  open: boolean;
  onOpenChange(open: boolean): void;
  noteId: string | null;
  revisions: JsonEntity[];
  onRestore(revision: JsonEntity): Promise<void>;
}) {
  const values = revisions
    .filter((revision) => text(revision, "noteId") === noteId)
    .sort((a, b) => number(b, "revisionVersion") - number(a, "revisionVersion"));

  return <Dialog
    open={open}
    onOpenChange={onOpenChange}
    title="版本历史"
    description="恢复不会改变文件夹、标签或收藏。"
  >
    {values.length ? <div className="max-h-[60vh] space-y-2 overflow-y-auto">
      {values.map((revision) => <div key={revision.meta.id} className="rounded-md border bg-card p-3">
        <div className="flex items-center gap-2">
          <History size={13} className="text-muted-foreground" />
          <div className="min-w-0 flex-1 truncate text-sm font-medium">{text(revision, "title", "无标题")}</div>
          <Badge>v{number(revision, "revisionVersion")}</Badge>
        </div>
        <div className="mt-1 text-[10px] text-muted-foreground">{new Date(revision.meta.createdAt).toLocaleString("zh-CN")}</div>
        <div className="mt-2 line-clamp-3 whitespace-pre-wrap text-xs leading-5 text-muted-foreground">
          {markdownSummary(text(revision, "contentMarkdown"), 260) || "空版本"}
        </div>
        <div className="mt-3 flex justify-end">
          <Button size="sm" variant="outline" onClick={() => void onRestore(revision)}>
            <RotateCcw size={13} />恢复此版本
          </Button>
        </div>
      </div>)}
    </div> : <EmptyState
      icon={<History size={22} />}
      title="还没有历史版本"
    />}
  </Dialog>;
}
