import { useEffect, useMemo, useState } from "react";
import { FolderCog, Loader2, Trash2 } from "lucide-react";
import { Button, Dialog, Input } from "../../components/ui";
import { text } from "../../lib/entities";
import type { JsonEntity } from "../../services/core";
import { flattenNoteFolders, folderDescendantIds } from "./folderTree";

export function NoteFolderSettings({
  open,
  onOpenChange,
  folder,
  folders,
  onSave,
  onDelete,
}: {
  open: boolean;
  onOpenChange(open: boolean): void;
  folder: JsonEntity | null;
  folders: JsonEntity[];
  onSave(name: string, parentFolderId: string | null): Promise<void>;
  onDelete(): Promise<void>;
}) {
  const [name, setName] = useState("");
  const [parentId, setParentId] = useState("");
  const [busy, setBusy] = useState<"save" | "delete" | "">("");
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open || !folder) return;
    setName(text(folder, "name"));
    setParentId(text(folder, "parentFolderId"));
    setError("");
    setBusy("");
  }, [folder?.meta.id, open]);

  const invalidParents = useMemo(() => {
    if (!folder) return new Set<string>();
    const values = folderDescendantIds(folders, folder.meta.id);
    values.add(folder.meta.id);
    return values;
  }, [folder, folders]);

  const rows = useMemo(
    () => flattenNoteFolders(folders).filter(({ folder: candidate }) => !invalidParents.has(candidate.meta.id)),
    [folders, invalidParents],
  );

  async function save() {
    if (!name.trim()) {
      setError("文件夹名称不能为空");
      return;
    }
    setBusy("save");
    setError("");
    try {
      await onSave(name.trim(), parentId || null);
      onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "保存文件夹失败");
    } finally {
      setBusy("");
    }
  }

  async function remove() {
    setBusy("delete");
    setError("");
    try {
      await onDelete();
      onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "删除文件夹失败");
    } finally {
      setBusy("");
    }
  }

  return <Dialog
    open={open}
    onOpenChange={onOpenChange}
    title="文件夹设置"
    description="调整名称或父级。删除文件夹时，笔记和直接子目录会迁移到它的父级，不会删除笔记内容。"
  >
    {folder ? <div className="space-y-4">
      <label className="block space-y-1.5 text-xs font-medium">名称
        <Input autoFocus value={name} onChange={(event) => setName(event.target.value)} />
      </label>
      <label className="block space-y-1.5 text-xs font-medium">父文件夹
        <select className="h-9 w-full rounded-md border bg-background px-3 text-sm" value={parentId} onChange={(event) => setParentId(event.target.value)}>
          <option value="">根目录</option>
          {rows.map(({ folder: candidate, depth, path }) => <option key={candidate.meta.id} value={candidate.meta.id}>{`${"— ".repeat(depth)}${path}`}</option>)}
        </select>
      </label>
      {error ? <div className="rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive">{error}</div> : null}
      <div className="flex items-center justify-between gap-2 border-t pt-3">
        <Button variant="ghost" className="text-destructive" disabled={busy !== ""} onClick={() => void remove()}>
          {busy === "delete" ? <Loader2 size={14} className="animate-spin" /> : <Trash2 size={14} />}删除文件夹
        </Button>
        <div className="flex gap-2">
          <Button variant="ghost" disabled={busy !== ""} onClick={() => onOpenChange(false)}>取消</Button>
          <Button disabled={busy !== ""} onClick={() => void save()}>
            {busy === "save" ? <Loader2 size={14} className="animate-spin" /> : <FolderCog size={14} />}保存
          </Button>
        </div>
      </div>
    </div> : null}
  </Dialog>;
}
