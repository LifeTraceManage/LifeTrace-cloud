import { API_BASE, browserFetch } from "../../services/core";

export interface NoteAttachment {
  id: string;
  domain: "notes_attachments";
  originalName: string;
  mimeType: string;
  sizeBytes: number;
  sha256: string;
  entityType: string | null;
  entityId: string | null;
  status: "pending" | "available" | "failed" | string;
  failureReason?: string | null;
  createdAt: string;
  updatedAt: string;
  availableAt?: string | null;
}

interface SignedTransfer {
  url: string;
  requiredHeaders: Record<string, string>;
  expiresSeconds: number;
}

interface PrepareResponse {
  file: NoteAttachment;
  deduplicated: boolean;
  upload: SignedTransfer | null;
}

interface FileListResponse {
  items: NoteAttachment[];
}

function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

async function parseError(response: Response): Promise<string> {
  try {
    const payload = await response.json() as { message?: string; error?: { message?: string } };
    return payload.message || payload.error?.message || `文件请求失败 (${response.status})`;
  } catch {
    return `文件请求失败 (${response.status})`;
  }
}

async function request<T>(path: string, csrfToken: string, init: RequestInit = {}): Promise<T> {
  const method = (init.method || "GET").toUpperCase();
  const headers = new Headers(init.headers);
  if (init.body !== undefined && !headers.has("content-type")) headers.set("content-type", "application/json");
  if (!["GET", "HEAD", "OPTIONS"].includes(method)) headers.set("x-csrf-token", csrfToken);
  const response = await browserFetch(apiUrl(path), { ...init, method, headers, credentials: "include" });
  if (!response.ok) throw new Error(await parseError(response));
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

async function sha256(file: File): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", await file.arrayBuffer());
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export class NoteFileApi {
  constructor(private readonly csrfToken: string) {}

  async list(noteId: string): Promise<NoteAttachment[]> {
    const params = new URLSearchParams({
      domain: "notes_attachments",
      entityType: "note.note",
      entityId: noteId,
      limit: "100",
    });
    return (await request<FileListResponse>(`/api/v1/files?${params}`, this.csrfToken)).items;
  }

  async upload(noteId: string, file: File): Promise<NoteAttachment> {
    const checksum = await sha256(file);
    const prepared = await request<PrepareResponse>("/api/v1/files", this.csrfToken, {
      method: "POST",
      body: JSON.stringify({
        domain: "notes_attachments",
        originalName: file.name,
        mimeType: file.type || "application/octet-stream",
        sizeBytes: file.size,
        sha256: checksum,
        entityType: "note.note",
        entityId: noteId,
      }),
    });

    if (prepared.file.status === "available") return prepared.file;
    if (!prepared.upload) throw new Error("文件服务未返回上传地址");

    const uploadHeaders = new Headers(prepared.upload.requiredHeaders);
    if (!uploadHeaders.has("content-type") && file.type) uploadHeaders.set("content-type", file.type);
    const uploaded = await fetch(prepared.upload.url, {
      method: "PUT",
      headers: uploadHeaders,
      body: file,
    });
    if (!uploaded.ok) {
      await request(`/api/v1/files/${encodeURIComponent(prepared.file.id)}/fail`, this.csrfToken, {
        method: "POST",
        body: JSON.stringify({ reason: `object upload failed (${uploaded.status})` }),
      }).catch(() => undefined);
      throw new Error(`附件上传失败 (${uploaded.status})`);
    }

    return request<NoteAttachment>(`/api/v1/files/${encodeURIComponent(prepared.file.id)}/complete`, this.csrfToken, {
      method: "POST",
      body: "{}",
    });
  }

  async downloadUrl(fileId: string): Promise<string> {
    const signed = await request<SignedTransfer>(`/api/v1/files/${encodeURIComponent(fileId)}/download-url`, this.csrfToken, {
      method: "POST",
      body: "{}",
    });
    const url = new URL(signed.url, window.location.origin);
    if (!["http:", "https:"].includes(url.protocol)) throw new Error("文件下载地址协议不受支持");
    return url.toString();
  }

  async remove(fileId: string): Promise<void> {
    await request(`/api/v1/files/${encodeURIComponent(fileId)}`, this.csrfToken, {
      method: "DELETE",
      body: "{}",
    });
  }
}

export function attachmentMarkdown(file: NoteAttachment): string {
  const safeName = file.originalName.replace(/[\[\]]/g, "");
  if (file.mimeType.startsWith("image/")) return `![${safeName}](attachment://${file.id})`;
  return `[${safeName}](attachment://${file.id})`;
}
