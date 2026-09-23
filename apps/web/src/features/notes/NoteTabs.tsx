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

  return <div className="scrollbar-thin mb-3 flex max-w-full items-center gap-1 overflow-x-auto border-b pb-2">
    {opened.map((note) => <div key={note.meta.id} className={cn("group flex h-8 max-w-48 shrink-0 items-center rounded-md border text-xs", activeId === note.meta.id ? "bg-accent text-accent-foreground" : "bg-card text-muted-foreground")}>
      <button className="min-w-0 flex-1 truncate px-2.5 text-left" onClick={() => onSelect(note.meta.id)}>{text(note, "title", "无标题")}</button>
      <button className="mr-1 rounded p-1 opacity-60 hover:bg-muted hover:opacity-100" aria-label={`关闭 ${text(note, "title", "无标题")}`} onClick={() => onClose(note.meta.id)}><X size={11} /></button>
    </div>)}
  </div>;
}
