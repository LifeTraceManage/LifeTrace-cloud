import { baseMeta, localDate, type JsonEntity } from "./types";

export interface HabitInput {
  name: string;
  activityType?: string;
  unit?: string;
  minimumTarget?: number | null;
  normalTarget?: number | null;
  targetPeriod?: "daily" | "weekly" | string;
  targetDays?: number[];
  scheduleType?: "daily" | "weekly" | "custom" | "interval" | "monthly" | string;
  startDate?: string | null;
  checkinMethod?: "manual" | "automatic" | string;
  icon?: string | null;
  color?: string | null;
  description?: string | null;
}

export function createHabitActivity(userId: string, deviceId: string, input: HabitInput): JsonEntity {
  const name = input.name.trim();
  if (!name) throw new Error("请输入项目名称");
  return {
    meta: baseMeta(userId, deviceId), name,
    activityType: input.activityType ?? "habit", unit: input.unit?.trim() || "次",
    minimumTarget: input.minimumTarget ?? null, normalTarget: input.normalTarget ?? 1,
    targetPeriod: input.targetPeriod ?? "daily", targetDays: input.targetDays ?? [],
    icon: input.icon ?? name.slice(0, 1), color: input.color ?? "#0f766e",
    scheduleType: input.scheduleType ?? "daily", startDate: input.startDate ?? localDate(),
    checkinMethod: input.checkinMethod ?? "manual",
    syncSource: "web", description: input.description?.trim() || null, isArchived: false,
  };
}

export function createHabitLog(userId: string, deviceId: string, activityId: string, value = 1, note = "", date = localDate()): JsonEntity {
  if (!activityId) throw new Error("请选择坚持项目");
  return {
    meta: baseMeta(userId, deviceId), activityId, logDate: date,
    value: Number.isFinite(value) ? value : 1, status: "completed",
    note: note.trim() || null, metadata: { source: "web" },
  };
}

export interface DailyReviewInput {
  reviewDate?: string;
  energy?: number | null;
  mood?: number | null;
  completionScore?: number | null;
  bestThing?: string;
  problem?: string;
  tomorrowPriority?: string;
  note?: string;
}

export function createDailyReview(userId: string, deviceId: string, input: DailyReviewInput, id?: string): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId, id), reviewDate: input.reviewDate ?? localDate(),
    energy: input.energy ?? null, mood: input.mood ?? null,
    completionScore: input.completionScore ?? null,
    bestThing: input.bestThing?.trim() || null, problem: input.problem?.trim() || null,
    tomorrowPriority: input.tomorrowPriority?.trim() || null, note: input.note?.trim() || null,
  };
}

export interface WorkoutInput {
  name?: string;
  occurredAt?: string;
  durationMinutes?: number;
  exerciseCount?: number;
  setCount?: number;
  volumeKg?: number | null;
  caloriesKcal?: number | null;
  source?: string;
}

export function createWorkout(userId: string, deviceId: string, input: WorkoutInput): JsonEntity {
  const occurredAt = input.occurredAt ?? new Date().toISOString();
  return {
    meta: baseMeta(userId, deviceId), source: input.source ?? "manual", sourceId: null,
    name: input.name?.trim() || "训练", occurredAt, localDate: localDate(new Date(occurredAt)),
    durationSeconds: Math.max(0, Math.round((input.durationMinutes ?? 0) * 60)),
    exerciseCount: Math.max(0, Math.round(input.exerciseCount ?? 0)),
    setCount: Math.max(0, Math.round(input.setCount ?? 0)), plannedSetCount: null,
    volumeKg: input.volumeKg ?? null, caloriesKcal: input.caloriesKcal ?? null, status: "completed",
  };
}

export function createWorkoutExercise(userId: string, deviceId: string, workoutId: string, name: string, sortOrder = 0, plannedSets = 0, completedSets = 0): JsonEntity {
  return { meta: baseMeta(userId, deviceId), workoutId, name: name.trim() || "训练动作", sortOrder, plannedSets, completedSets };
}

export function createWorkoutSet(userId: string, deviceId: string, exerciseId: string, setNumber: number, weightKg: number | null, reps: number | null): JsonEntity {
  return { meta: baseMeta(userId, deviceId), exerciseId, setNumber, weightKg, reps, completed: true };
}

export function createWorkoutImport(userId: string, deviceId: string, shareUrl: string): JsonEntity {
  return { meta: baseMeta(userId, deviceId), source: "xunji", shareUrl: shareUrl.trim() || null, status: "pending", parser: null, parserVersion: null, error: null, workoutId: null };
}

export function createTrainingNote(userId: string, deviceId: string, title: string, content: string, workoutId: string | null = null): JsonEntity {
  return { meta: baseMeta(userId, deviceId), title: title.trim() || "训练笔记", content: content.trim(), workoutId, source: "manual", noteDate: localDate() };
}

export function createNoteFolder(
  userId: string,
  deviceId: string,
  name: string,
  sortOrder = 0,
  parentFolderId: string | null = null,
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    name: name.trim(),
    icon: "folder",
    color: "#8a765b",
    parentFolderId,
    sortOrder,
  };
}

export function createNoteTag(userId: string, deviceId: string, name: string): JsonEntity {
  return { meta: baseMeta(userId, deviceId), name: name.trim(), color: "#49715d" };
}

export function createNoteRevision(
  userId: string,
  deviceId: string,
  note: JsonEntity,
  revisionVersion: number,
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    noteId: note.meta.id,
    revisionVersion,
    title: typeof note.title === "string" ? note.title : null,
    contentJson: note.contentJson ?? {},
    contentHtml: typeof note.contentHtml === "string" ? note.contentHtml : "",
    contentMarkdown: typeof note.contentMarkdown === "string"
      ? note.contentMarkdown
      : typeof note.contentText === "string"
        ? note.contentText
        : "",
  };
}

export function createNoteTagRelation(userId: string, deviceId: string, noteId: string, tagId: string): JsonEntity {
  return { meta: baseMeta(userId, deviceId, `${noteId}:${tagId}`), noteId, tagId };
}

export function createNoteRelation(userId: string, deviceId: string, noteId: string, targetNoteId: string): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId, `${noteId}:${targetNoteId}`),
    noteId,
    entityType: "note.note",
    entityId: targetNoteId,
    relationType: "wiki_link",
  };
}

export interface NoteContent { html: string; text: string; json: unknown; markdown?: string; }

function escapeHtml(value: string): string {
  return value.replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[character] ?? character);
}

export function createNote(userId: string, deviceId: string, title: string, content: string | NoteContent, folderId: string | null = null): JsonEntity {
  const value: NoteContent = typeof content === "string"
    ? { html: content.trim() ? `<p>${escapeHtml(content.trim()).replace(/\n/g, "<br>")}</p>` : "", text: content.trim(), json: { type: "doc", content: content.trim() }, markdown: content.trim() }
    : content;
  return {
    meta: baseMeta(userId, deviceId), noteType: "quick", title: title.trim() || null,
    contentJson: value.json, contentHtml: value.html, contentText: value.text,
    contentMarkdown: value.markdown ?? value.text, summary: value.text.slice(0, 160),
    isPinned: false, isFavorite: false, isArchived: false, folderId,
    aiSummary: null, aiTags: null, embeddingStatus: null, lastAiProcessedAt: null,
  };
}

export function createPreference(userId: string, deviceId: string, preferenceKey: string, value: unknown): JsonEntity {
  return { meta: baseMeta(userId, deviceId), preferenceKey, value };
}

export function createEntityLink(
  userId: string,
  deviceId: string,
  sourceType: string,
  sourceId: string,
  relationType: string,
  targetType: string,
  targetId: string,
  metadata: unknown = null,
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    source: { entityType: sourceType, entityId: sourceId },
    target: { entityType: targetType, entityId: targetId },
    relationType,
    metadata,
  };
}

export function createFileMetadata(userId: string, deviceId: string, file: { name: string; type: string; size: number; sha256: string }): JsonEntity {
  return { meta: baseMeta(userId, deviceId), originalName: file.name, mimeType: file.type || "application/octet-stream", sizeBytes: file.size, sha256: file.sha256, storageState: "pending_upload", createdByDevice: deviceId };
}
