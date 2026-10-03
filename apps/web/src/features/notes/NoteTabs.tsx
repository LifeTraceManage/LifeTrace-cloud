import { X } from "lucide-react";
import { cn } from "../../components/ui";
import { text } from "../../lib/entities";
import type { JsonEntity } from "../../services/core";

export function NoteTabs({
  notes,
  openedIds,
  activeId,
  onSelect,
  onClose,
}: {
  notes: JsonEntity[];
  openedIds: string[];
  activeId: string | null;
  onSelect(id: string): void;
  onClose(id: string): void;
}) {
  const byId = new Map(notes.map((note) => [note.meta.id, note]));
  const opened = openedIds.map((id) => byId.get(id)).filter((note): note is JsonEntity => Boolean(note));
  if (!opened.length) return null;

  return <div className="scrollbar-thin mb-2 flex max-w-full items-end gap-0 overflow-x-auto border-b" data-testid="note-tabs">
    {opened.map((note) => <div key={note.meta.id} className={cn("group flex h-9 max-w-48 shrink-0 items-center border-b-2 text-xs transition-colors", activeId === note.meta.id ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:bg-muted/30 hover:text-foreground")}>
      <button className="min-w-0 flex-1 truncate px-2.5 text-left" onClick={() => onSelect(note.meta.id)}>{text(note, "title", "无标题")}</button>
      <button className="mr-1 p-1 opacity-60 hover:bg-muted/60 hover:opacity-100" aria-label={`关闭 ${text(note, "title", "无标题")}`} onClick={() => onClose(note.meta.id)}><X size={11} /></button>
    </div>)}
  </div>;
}
