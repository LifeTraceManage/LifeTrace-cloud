import { useCallback, useEffect, useRef, useState } from "react";
import { Download, FilePlus2, Loader2, Paperclip, Plus, Trash2 } from "lucide-react";
import { Badge, Button } from "../../components/ui";
import { useApp } from "../../app/AppContext";
import { NoteFileApi, attachmentMarkdown, type NoteAttachment } from "./files";

function sizeLabel(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.ceil(bytes / 1024)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function NoteAttachments({
  noteId,
  onInsertMarkdown,
}: {
  noteId: string;
  onInsertMarkdown(value: string): void;
}) {
  const { session } = useApp();
  const [items, setItems] = useState<NoteAttachment[]>([]);
  const [loading, setLoading] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const load = useCallback(async () => {
    if (!session?.csrfToken) return;
    setLoading(true);
    setError("");
    try {
      setItems(await new NoteFileApi(session.csrfToken).list(noteId));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "无法加载附件");
    } finally {
      setLoading(false);
    }
  }, [noteId, session?.csrfToken]);

  useEffect(() => {
    void load();
  }, [load]);

  async function upload(file: File) {
    if (!session?.csrfToken) return;
    setUploading(true);
    setError("");
    try {
      const uploaded = await new NoteFileApi(session.csrfToken).upload(noteId, file);
      setItems((current) => [uploaded, ...current.filter((item) => item.id !== uploaded.id)]);
      onInsertMarkdown(attachmentMarkdown(uploaded));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "附件上传失败");
    } finally {
      setUploading(false);
      if (inputRef.current) inputRef.current.value = "";
    }
  }

  async function download(item: NoteAttachment) {
    if (!session?.csrfToken) return;
    try {
      const url = await new NoteFileApi(session.csrfToken).downloadUrl(item.id);
      window.open(url, "_blank", "noopener,noreferrer");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "无法获取下载地址");
    }
  }

  async function remove(item: NoteAttachment) {
    if (!session?.csrfToken) return;
    try {
      await new NoteFileApi(session.csrfToken).remove(item.id);
      setItems((current) => current.filter((candidate) => candidate.id !== item.id));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "删除附件失败");
    }
  }

  return <section className="border-t pt-4">
    <div className="mb-2 flex items-center justify-between">
      <div className="flex items-center gap-2 text-xs font-semibold"><Paperclip size={14} />Attachments</div>
      <div className="flex items-center gap-2">
        <Badge>{items.length}</Badge>
        <input
          ref={inputRef}
          type="file"
          className="hidden"
          accept="image/*,audio/*,video/*,application/pdf,text/plain,text/markdown,application/zip"
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void upload(file);
          }}
        />
        <Button size="sm" variant="outline" disabled={uploading} onClick={() => inputRef.current?.click()}>
          {uploading ? <Loader2 size={13} className="animate-spin" /> : <Plus size={13} />}附件
        </Button>
      </div>
    </div>
    {loading ? <div className="flex items-center gap-2 py-2 text-xs text-muted-foreground"><Loader2 size={13} className="animate-spin" />加载附件…</div> : null}
    {error ? <div className="mb-2 rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive">{error}</div> : null}
    {!loading && !items.length ? <div className="rounded-md border border-dashed p-3 text-xs leading-5 text-muted-foreground"><FilePlus2 size={15} className="mb-1" />上传后会自动向 Markdown 插入 <code>attachment://</code> 引用；文件本体由 LifeTrace File API 管理。</div> : null}
    <div className="space-y-1">
      {items.map((item) => <div key={item.id} className="flex items-center gap-2 rounded-md border bg-card px-2.5 py-2 text-xs">
        <Paperclip size={13} className="shrink-0 text-muted-foreground" />
        <div className="min-w-0 flex-1"><div className="truncate font-medium">{item.originalName}</div><div className="mt-0.5 text-[10px] text-muted-foreground">{item.mimeType} · {sizeLabel(item.sizeBytes)}</div></div>
        <Button size="icon" variant="ghost" className="h-7 w-7" onClick={() => onInsertMarkdown(attachmentMarkdown(item))} aria-label="插入附件引用"><Plus size={13} /></Button>
        <Button size="icon" variant="ghost" className="h-7 w-7" onClick={() => void download(item)} aria-label="下载附件"><Download size={13} /></Button>
        <Button size="icon" variant="ghost" className="h-7 w-7" onClick={() => void remove(item)} aria-label="删除附件"><Trash2 size={13} /></Button>
      </div>)}
    </div>
  </section>;
}
