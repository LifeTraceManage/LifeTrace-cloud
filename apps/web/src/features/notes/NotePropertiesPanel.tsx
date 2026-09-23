import { useEffect, useState } from "react";
import { Braces, CalendarClock, Star, Tag } from "lucide-react";
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

export function NotePropertiesPanel({
  note,
  title,
  properties,
  tagNames,
  favorite,
  disabled = false,
  onChange,
}: {
  note: JsonEntity;
  title: string;
  properties: NoteProperties;
  tagNames: string[];
  favorite: boolean;
  disabled?: boolean;
  onChange(next: NoteProperties): void;
}) {
  const [aliasesInput, setAliasesInput] = useState(aliasText(properties.aliases));

  useEffect(() => {
    setAliasesInput(aliasText(properties.aliases));
  }, [properties.aliases.join("\u0000")]);

  return <section className="rounded-md border bg-card/35 p-3">
    <div className="mb-3 flex items-center gap-2 text-xs font-semibold"><Braces size={14} />Properties</div>
    <div className="space-y-3">
      <div className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">title</span>
        <span className="truncate font-medium">{title.trim() || "无标题"}</span>
      </div>
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
        <span className="pt-1 text-muted-foreground">tags</span>
        <div className="flex flex-wrap gap-1">
          {tagNames.length ? tagNames.map((name) => <Badge key={name}>#{name}</Badge>) : <span className="pt-1 text-muted-foreground">—</span>}
        </div>
      </div>
      <div className="grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 text-xs">
        <span className="text-muted-foreground">favorite</span>
        <span className="flex items-center gap-1.5">{favorite ? <Star size={12} className="fill-current text-warning" /> : <Star size={12} />} {favorite ? "true" : "false"}</span>
      </div>
      <div className="border-t pt-2 text-[10px] leading-5 text-muted-foreground">
        <div className="flex items-center gap-1.5"><CalendarClock size={11} />created · {new Date(note.meta.createdAt).toLocaleString("zh-CN")}</div>
        <div className="flex items-center gap-1.5"><CalendarClock size={11} />updated · {new Date(note.meta.updatedAt).toLocaleString("zh-CN")}</div>
        <div className="mt-1 flex items-center gap-1.5"><Tag size={11} />YAML frontmatter 导入时会映射到这些结构化属性；保存后正文只保留 Markdown 内容。</div>
      </div>
    </div>
  </section>;
}
