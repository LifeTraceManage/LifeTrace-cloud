import { Link2, Link2Off, Undo2 } from "lucide-react";
import { Badge, EmptyState } from "../../components/ui";
import { text } from "../../lib/entities";
import type { JsonEntity } from "../../services/core";
import { extractWikiLinks } from "./markdown";
import { noteAliases } from "./properties";

function resolveNote(notes: JsonEntity[], title: string): JsonEntity | null {
  const key = title.trim().toLocaleLowerCase("zh-CN");
  return notes.find((note) => {
    if (text(note, "title").trim().toLocaleLowerCase("zh-CN") === key) return true;
    return noteAliases(note.contentJson).some((alias) => alias.toLocaleLowerCase("zh-CN") === key);
  }) ?? null;
}

export function NotesKnowledgePanel({
  note,
  notes,
  relations,
  onOpenNote,
}: {
  note: JsonEntity;
  notes: JsonEntity[];
  relations: JsonEntity[];
  onOpenNote(id: string): void;
}) {
  const links = extractWikiLinks(text(note, "contentMarkdown", text(note, "contentText")));
  const outgoing = links.map((link) => ({ ...link, note: resolveNote(notes, link.target) }));
  const backlinks = relations
    .filter((relation) =>
      text(relation, "entityType") === "note.note"
      && text(relation, "entityId") === note.meta.id
      && text(relation, "relationType", "wiki_link") === "wiki_link"
    )
    .map((relation) => notes.find((candidate) => candidate.meta.id === text(relation, "noteId")))
    .filter((candidate): candidate is JsonEntity => Boolean(candidate));

  return <aside className="space-y-5 border-t pt-4 xl:border-l xl:border-t-0 xl:pl-4 xl:pt-0">
    <section>
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-semibold"><Link2 size={14} />Links</div>
        <Badge>{outgoing.length}</Badge>
      </div>
      {outgoing.length ? <div className="space-y-1">
        {outgoing.map((link, index) => link.note ? <button key={`${link.raw}:${index}`} onClick={() => onOpenNote(link.note!.meta.id)} className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs hover:bg-muted"><Link2 size={12} className="shrink-0 text-primary" /><span className="truncate">{link.label}</span></button> : <div key={`${link.raw}:${index}`} className="flex items-center gap-2 rounded-md px-2 py-1.5 text-xs text-muted-foreground"><Link2Off size={12} className="shrink-0" /><span className="truncate">{link.target}</span><span className="ml-auto text-[10px]">未解析</span></div>)}
      </div> : <div className="text-xs leading-5 text-muted-foreground">在正文中输入 <code>[[Note Name]]</code> 建立知识链接。</div>}
    </section>

    <section>
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-semibold"><Undo2 size={14} />Backlinks</div>
        <Badge>{backlinks.length}</Badge>
      </div>
      {backlinks.length ? <div className="space-y-1">{backlinks.map((source) => <button key={source.meta.id} onClick={() => onOpenNote(source.meta.id)} className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs hover:bg-muted"><Undo2 size={12} className="shrink-0 text-primary" /><span className="truncate">{text(source, "title", "无标题")}</span></button>)}</div> : <EmptyState title="暂无反向链接" />}
    </section>
  </aside>;
}
