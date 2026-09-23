import { useMemo } from "react";
import { Network } from "lucide-react";
import { EmptyState } from "../../components/ui";
import { text } from "../../lib/entities";
import type { JsonEntity } from "../../services/core";

type Point = { id: string; x: number; y: number; title: string; favorite: boolean };

export function NotesGraphView({
  notes,
  relations,
  onOpenNote,
}: {
  notes: JsonEntity[];
  relations: JsonEntity[];
  onOpenNote(id: string): void;
}) {
  const graph = useMemo(() => {
    const visible = notes.slice(0, 80);
    const ids = new Set(visible.map((note) => note.meta.id));
    const centerX = 380;
    const centerY = 230;
    const radius = Math.min(185, 90 + visible.length * 2.3);
    const points: Point[] = visible.map((note, index) => {
      const angle = visible.length <= 1 ? 0 : (Math.PI * 2 * index) / visible.length - Math.PI / 2;
      return {
        id: note.meta.id,
        x: visible.length === 1 ? centerX : centerX + Math.cos(angle) * radius,
        y: visible.length === 1 ? centerY : centerY + Math.sin(angle) * radius,
        title: text(note, "title", "无标题"),
        favorite: note.isFavorite === true,
      };
    });
    const pointById = new Map(points.map((point) => [point.id, point]));
    const edges = relations
      .filter((relation) =>
        text(relation, "relationType") === "wiki_link"
        && text(relation, "entityType") === "note.note"
        && ids.has(text(relation, "noteId"))
        && ids.has(text(relation, "entityId"))
      )
      .map((relation) => ({
        source: pointById.get(text(relation, "noteId")),
        target: pointById.get(text(relation, "entityId")),
      }))
      .filter((edge): edge is { source: Point; target: Point } => Boolean(edge.source && edge.target));
    return { points, edges };
  }, [notes, relations]);

  if (!graph.points.length) {
    return <EmptyState icon={<Network size={22} />} title="知识图谱为空" description="创建笔记并使用 [[Wiki Link]] 后会形成关系图。" />;
  }

  return <div className="overflow-x-auto rounded-md border bg-card/30">
    <svg viewBox="0 0 760 460" className="min-h-[420px] min-w-[680px] w-full" role="img" aria-label="Notes Wiki Link graph">
      <g opacity="0.45">
        {graph.edges.map((edge, index) => <line
          key={index}
          x1={edge.source.x}
          y1={edge.source.y}
          x2={edge.target.x}
          y2={edge.target.y}
          stroke="currentColor"
          strokeWidth="1.2"
          className="text-muted-foreground"
        />)}
      </g>
      {graph.points.map((point) => <g
        key={point.id}
        className="cursor-pointer"
        onClick={() => onOpenNote(point.id)}
        role="button"
        tabIndex={0}
        onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") onOpenNote(point.id); }}
      >
        <circle cx={point.x} cy={point.y} r={point.favorite ? 8 : 6} className={point.favorite ? "fill-warning" : "fill-primary"} />
        <text
          x={point.x}
          y={point.y + 18}
          textAnchor="middle"
          className="fill-foreground text-[10px]"
        >{point.title.length > 18 ? `${point.title.slice(0, 17)}…` : point.title}</text>
      </g>)}
    </svg>
    {notes.length > 80 ? <div className="border-t px-3 py-2 text-[10px] text-muted-foreground">为保持交互流畅，当前图只展示最近的 80 篇笔记。</div> : null}
  </div>;
}