import { useMemo, useState } from "react";
import { FilePlus2, Folder, FolderInput, Inbox, NotebookPen, Search, Tag, Trash2 } from "lucide-react";
import { Dialog, Input } from "../../components/ui";
import { text } from "../../lib/entities";
import type { JsonEntity } from "../../services/core";

type Scope = "all" | "inbox" | `folder:${string}` | `tag:${string}`;

function matches(needle: string, ...values: string[]): boolean {
  return !needle || values.some((value) => value.toLocaleLowerCase("zh-CN").includes(needle));
}

export function NotesCommandPalette({
  open,
  onOpenChange,
  notes,
  folders,
  tags,
  selectedId,
  selectedTagIds,
  onNewNote,
  onOpenNote,
  onScope,
  onMoveSelected,
  onArchiveSelected,
  onAddSelectedTag,
}: {
  open: boolean;
  onOpenChange(open: boolean): void;
  notes: JsonEntity[];
  folders: JsonEntity[];
  tags: JsonEntity[];
  selectedId: string | null;
  selectedTagIds: Set<string>;
  onNewNote(): void;
  onOpenNote(id: string): void;
  onScope(scope: Scope): void;
  onMoveSelected(folderId: string): void;
  onArchiveSelected(): void;
  onAddSelectedTag(tagId: string): void;
}) {
  const [query, setQuery] = useState("");
  const needle = query.trim().toLocaleLowerCase("zh-CN");
  const selected = notes.find((note) => note.meta.id === selectedId) ?? null;

  const filteredNotes = useMemo(() => notes
    .filter((note) => matches(needle, text(note, "title", "无标题"), text(note, "contentText")))
    .slice(0, 12), [needle, notes]);

  function run(action: () => void) {
    action();
    setQuery("");
    onOpenChange(false);
  }

  const showNew = matches(needle, "new note", "新建笔记", "create note");
  const showSearch = matches(needle, "search notes", "搜索笔记", "all notes");
  const showDelete = selected && matches(needle, "delete note", "trash note", "删除笔记", "移到废纸篓");
  const availableTags = tags.filter((tag) => !selectedTagIds.has(tag.meta.id))
    .filter((tag) => matches(needle, "add tag", "添加标签", text(tag, "name")))
    .slice(0, 8);
  const moveFolders = folders
    .filter((folder) => matches(needle, "move note", "移动笔记", text(folder, "name")))
    .slice(0, 10);

  return <Dialog
    open={open}
    onOpenChange={(next) => {
      if (!next) setQuery("");
      onOpenChange(next);
    }}
    title="Notes 命令面板"
    description="Ctrl / Cmd + P 打开。支持创建、搜索、打开、移动、删除和标签操作。"
  >
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" size={15} />
      <Input autoFocus className="pl-9" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="输入命令、笔记、文件夹或标签…" />
    </div>

    <div className="mt-3 max-h-[56vh] space-y-4 overflow-y-auto">
      {(showNew || showSearch || showDelete) ? <div>
        <div className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Commands</div>
        {showNew ? <button className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(onNewNote)}><FilePlus2 size={15} className="text-primary" /><span>New Note · 新建笔记</span></button> : null}
        {showSearch ? <button className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(() => onScope("all"))}><Search size={15} /><span>Search Notes · 搜索全部笔记</span></button> : null}
        {showDelete ? <button className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm text-destructive hover:bg-destructive/10" onClick={() => run(onArchiveSelected)}><Trash2 size={15} /><span>Delete Note · 移到废纸篓</span></button> : null}
      </div> : null}

      {selected ? <div>
        <div className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Move current note</div>
        {matches(needle, "move note", "移动笔记", "inbox") ? <button className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(() => onMoveSelected(""))}><Inbox size={14} /><span>移动到 Inbox</span></button> : null}
        {moveFolders.map((folder) => <button key={folder.meta.id} className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(() => onMoveSelected(folder.meta.id))}><FolderInput size={14} /><span className="truncate">移动到 {text(folder, "name", "未命名")}</span></button>)}
      </div> : null}

      {selected && availableTags.length ? <div>
        <div className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Add tag</div>
        {availableTags.map((tag) => <button key={tag.meta.id} className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(() => onAddSelectedTag(tag.meta.id))}><Tag size={14} /><span className="truncate">添加 #{text(tag, "name")}</span></button>)}
      </div> : null}

      {filteredNotes.length ? <div>
        <div className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Open Note</div>
        {filteredNotes.map((note) => <button key={note.meta.id} className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(() => onOpenNote(note.meta.id))}><NotebookPen size={14} /><span className="truncate">{text(note, "title", "无标题")}</span></button>)}
      </div> : null}

      {folders.filter((folder) => matches(needle, "folder", "文件夹", text(folder, "name"))).length ? <div>
        <div className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Open Folder</div>
        {folders.filter((folder) => matches(needle, "folder", "文件夹", text(folder, "name"))).slice(0, 8).map((folder) => <button key={folder.meta.id} className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(() => onScope(`folder:${folder.meta.id}`))}><Folder size={14} /><span className="truncate">{text(folder, "name")}</span></button>)}
      </div> : null}

      {tags.filter((tag) => matches(needle, "tag", "标签", text(tag, "name"))).length ? <div>
        <div className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Open Tag</div>
        {tags.filter((tag) => matches(needle, "tag", "标签", text(tag, "name"))).slice(0, 8).map((tag) => <button key={tag.meta.id} className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted" onClick={() => run(() => onScope(`tag:${tag.meta.id}`))}><Tag size={14} /><span className="truncate">#{text(tag, "name")}</span></button>)}
      </div> : null}
    </div>
  </Dialog>;
}
