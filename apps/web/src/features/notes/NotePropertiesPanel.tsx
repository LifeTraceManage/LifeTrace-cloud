import { useEffect, useState } from "react";
import { Braces, CalendarClock, FileText, Folder, Star, Tag, X } from "lucide-react";
import { Badge, Input } from "../../components/ui";
import type { JsonEntity } from "../../services/core";
import type { NoteProperties } from "./properties";

function aliasText(values: string[]): string {
  return values.join(", ");
}

function parseAliases(value: string): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const raw of value.split(/[,;\n]/)) {
    const alias = raw.trim();
    if (!alias) continue;
    const key = alias.toLocaleLowerCase("zh-CN");
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(alias);
  }
  return result;
}

export interface NotePropertyFolder {
  id: string;
  name: string;
  depth: number;
}

export interface NotePropertyTag {
  id: string;
  name: string;
  selected: boolean;
}

export function NotePropertiesPanel({
  note,
  title,
  properties,
  tagNames,
  favorite,
  disabled = false,
  wordCount,
  folderId,
  folders = [],
  tags = [],
  onMoveFolder,
  onAddTag,
  onRemoveTag,
  onChange,
}: {
  note: JsonEntity;
  title: string;
  properties: NoteProperties;
  tagNames: string[];
  favorite: boolean;
  disabled?: boolean;
  wordCount?: number;
  folderId?: string;
  folders?: NotePropertyFolder[];
  tags?: NotePropertyTag[];
  onMoveFolder?(folderId: string): void;
  onAddTag?(tagId: string): void;
  onRemoveTag?(tagId: string): void;
  onChange(next: NoteProperties): void;
}) {
  const [aliasesInput, setAliasesInput] = useState(aliasText(properties.aliases));

  useEffect(() => {
    setAliasesInput(aliasText(properties.aliases));
  }, [properties.aliases.join("\u0000")]);

  const selectedTags = tags.filter((item) => item.selected);
  const availableTags = tags.filter((item) => !item.selected);

  return <section className="rounded-md border bg-card/35" data-testid="note-properties">
    <div className="flex items-center gap-2 border-b px-3 py-2.5 text-xs font-semibold">
      <Braces size={14} />
      Properties
    </div>
    <div className="space-y-3 p-3">
      <div className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">title</span>
        <span className="truncate font-medium">{title.trim() || "无标题"}</span>
      </div>

      {typeof wordCount === "number" ? <div className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">字数</span>
        <span className="flex items-center gap-1.5"><FileText size={12} />{wordCount}</span>
      </div> : null}

      {onMoveFolder ? <label className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="flex items-center gap-1 text-muted-foreground"><Folder size={12} />文件夹</span>
        <select
          className="h-8 min-w-0 rounded-md border bg-background px-2 text-xs"
          value={folderId ?? ""}
          disabled={disabled}
          onChange={(event) => onMoveFolder(event.target.value)}
          aria-label="移动笔记到文件夹"
        >
          <option value="">Inbox</option>
          {folders.map((folder) => <option key={folder.id} value={folder.id}>{`${"— ".repeat(folder.depth)}${folder.name}`}</option>)}
        </select>
      </label> : null}

      <label className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">status</span>
        <Input
          className="h-8 text-xs"
          disabled={disabled}
          value={properties.status}
          onChange={(event) => onChange({ ...properties, status: event.target.value })}
          placeholder="research / draft / reference…"
        />
      </label>
      <label className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">source</span>
        <Input
          className="h-8 text-xs"
          disabled={disabled}
          value={properties.source}
          onChange={(event) => onChange({ ...properties, source: event.target.value })}
          placeholder="paper / web / mail…"
        />
      </label>
      <label className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">aliases</span>
        <Input
          className="h-8 text-xs"
          disabled={disabled}
          value={aliasesInput}
          onChange={(event) => setAliasesInput(event.target.value)}
          onBlur={() => {
            const aliases = parseAliases(aliasesInput);
            setAliasesInput(aliasText(aliases));
            onChange({ ...properties, aliases });
          }}
          placeholder="别名用逗号分隔"
        />
      </label>

      <div className="grid grid-cols-[72px_minmax(0,1fr)] items-start gap-2 text-xs">
        <span className="flex items-center gap-1 pt-1 text-muted-foreground"><Tag size={12} />tags</span>
        <div className="min-w-0 space-y-1.5">
          <div className="flex flex-wrap gap-1">
            {selectedTags.length ? selectedTags.map((item) => <Badge key={item.id} className="gap-1">
              #{item.name}
              {onRemoveTag && !disabled ? <button
                type="button"
                className="rounded-sm text-muted-foreground hover:text-foreground"
                aria-label={`移除标签 ${item.name}`}
                onClick={() => onRemoveTag(item.id)}
              ><X size={10} /></button> : null}
            </Badge>) : tagNames.length ? tagNames.map((name) => <Badge key={name}>#{name}</Badge>) : <span className="pt-1 text-muted-foreground">—</span>}
          </div>
          {onAddTag && availableTags.length && !disabled ? <select
            className="h-7 max-w-full rounded-md border bg-background px-2 text-[11px] text-muted-foreground"
            value=""
            onChange={(event) => {
              if (event.target.value) onAddTag(event.target.value);
            }}
            aria-label="添加标签"
          >
            <option value="">+ 添加标签</option>
            {availableTags.map((item) => <option key={item.id} value={item.id}>#{item.name}</option>)}
          </select> : null}
        </div>
      </div>

      <div className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">favorite</span>
        <span className="flex items-center gap-1.5">{favorite ? <Star size={12} className="fill-current text-warning" /> : <Star size={12} />} {favorite ? "true" : "false"}</span>
      </div>

      <div className="border-t pt-2 text-[10px] leading-5 text-muted-foreground">
        <div className="flex items-center gap-1.5"><CalendarClock size={11} />created · {new Date(note.meta.createdAt).toLocaleString("zh-CN")}</div>
        <div className="flex items-center gap-1.5"><CalendarClock size={11} />updated · {new Date(note.meta.updatedAt).toLocaleString("zh-CN")}</div>
      </div>
    </div>
  </section>;
}
