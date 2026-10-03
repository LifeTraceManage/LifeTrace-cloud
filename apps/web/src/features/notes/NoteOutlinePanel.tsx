import { useMemo } from "react";
import { ListTree } from "lucide-react";
import { cn } from "../../components/ui";
import { extractMarkdownHeadings, type MarkdownHeading } from "./markdown";

export function NoteOutlinePanel({
  markdown,
  onSelect,
}: {
  markdown: string;
  onSelect(heading: MarkdownHeading): void;
}) {
  const headings = useMemo(() => extractMarkdownHeadings(markdown), [markdown]);

  return <section className="py-3" data-testid="note-outline" data-inspector-section="outline">
    <div className="flex items-center gap-2 px-1 py-1.5 text-xs font-semibold">
      <ListTree size={14} />
      大纲
      <span className="ml-auto text-[10px] font-normal text-muted-foreground">{headings.length}</span>
    </div>
    <div className="p-1 pt-2">
      {headings.length ? <nav aria-label="笔记大纲" className="space-y-0.5">
        {headings.map((heading) => <button
          key={`${heading.line}:${heading.text}`}
          type="button"
          onClick={() => onSelect(heading)}
          className={cn(
            "block w-full truncate rounded px-2 py-1.5 text-left text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground",
            heading.level === 1 && "font-semibold text-foreground",
          )}
          style={{ paddingLeft: `${8 + Math.max(0, heading.level - 1) * 12}px` }}
          title={heading.text}
        >
          {heading.text}
        </button>)}
      </nav> : <div className="px-2 py-4 text-center text-[11px] text-muted-foreground">使用 # 标题后，大纲会立即更新</div>}
    </div>
  </section>;
}
