import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useSearchParams } from "react-router-dom";
import {
  ArrowLeft, CalendarDays, Check, CheckSquare2, Clock3, Command, Folder, FolderPlus, History, Inbox, Loader2, Network, NotebookPen,
  Plus, RotateCcw, Save, Search, Settings2, Star, Tag, Trash2, X,
} from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Button, Dialog, EmptyState, Input, cn } from "../../components/ui";
import { entities, number, text, todayKey } from "../../lib/entities";
import { WorkspaceShell } from "../../layouts/WorkspaceShell";
import {
  createEntityLink, createExecutionTask, createNote, createNoteFolder, createNoteRelation, createNoteRevision, createNoteTag, createNoteTagRelation, type JsonEntity,
} from "../../services/core";
import { flattenNoteFolders } from "./folderTree";
import { appendMarkdownLink, markdownSummary, plainTextFromMarkdown, uniqueWikiTargets } from "./markdown";
import { NoteAttachments } from "./NoteAttachments";
import { NoteFolderSettings } from "./NoteFolderSettings";
import { NotePropertiesPanel } from "./NotePropertiesPanel";
import { NoteRevisionHistory } from "./NoteRevisionHistory";
import { NoteTabs } from "./NoteTabs";
import { NotesCommandPalette } from "./NotesCommandPalette";
import { NotesGraphView } from "./NotesGraphView";
import { NotesKnowledgePanel } from "./NotesKnowledgePanel";
import {
  mergeNoteProperties, noteAliases, parseFrontmatter, storedNoteProperties, type NoteProperties,
} from "./properties";
import { VditorEditor } from "./VditorEditor";

type SaveState = "saved" | "saving" | "dirty" | "error";
type BuiltinScope = "inbox" | "all" | "recent" | "favorites" | "trash";
type NotesScope = BuiltinScope | `folder:${string}` | `tag:${string}`;

function noteMarkdown(note: JsonEntity): string {
  return text(note, "contentMarkdown", text(note, "contentText"));
}

function notePayload(
  note: JsonEntity,
  title: string,
  markdown: string,
  properties: NoteProperties,
  favorite: boolean,
): JsonEntity {
  const normalizedTitle = title.trim();
  const plainText = plainTextFromMarkdown(markdown);
  const currentJson = note.contentJson && typeof note.contentJson === "object" && !Array.isArray(note.contentJson)
    ? note.contentJson as Record<string, unknown>
    : {};
  return {
    ...note,
    title: normalizedTitle || null,
    contentText: plainText,
    contentMarkdown: markdown,
    contentHtml: "",
    contentJson: {
      ...currentJson,
      type: "markdown",
      source: markdown,
      editor: "vditor",
      properties: {
        status: properties.status.trim(),
        source: properties.source.trim(),
        aliases: properties.aliases,
      },
    },
    summary: plainText.slice(0, 160),
    isFavorite: favorite,
  };
}

function scopeId(scope: NotesScope, prefix: "folder" | "tag"): string | null {
  return scope.startsWith(`${prefix}:`) ? scope.slice(prefix.length + 1) : null;
}

export function NotesPage() {
  const { state, session, upsert, remove } = useApp();
  const [searchParams, setSearchParams] = useSearchParams();
  const notes = useMemo(
    () => entities(state, "note.note").sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt)),
    [state],
  );
  const folders = useMemo(
    () => entities(state, "note.folder").sort((a, b) => number(a, "sortOrder") - number(b, "sortOrder") || text(a, "name").localeCompare(text(b, "name"), "zh-CN")),
    [state],
  );
  const tags = useMemo(
    () => entities(state, "note.tag").sort((a, b) => text(a, "name").localeCompare(text(b, "name"), "zh-CN")),
    [state],
  );
  const tagRelations = useMemo(() => entities(state, "note.tag_relation"), [state]);
  const noteRelations = useMemo(() => entities(state, "note.relation"), [state]);
  const revisions = useMemo(() => entities(state, "note.revision"), [state]);

  const activeNotes = useMemo(() => notes.filter((item) => item.isArchived !== true), [notes]);
  const archivedNotes = useMemo(() => notes.filter((item) => item.isArchived === true), [notes]);
  const folderById = useMemo(() => new Map(folders.map((folder) => [folder.meta.id, folder])), [folders]);
  const tagById = useMemo(() => new Map(tags.map((tagItem) => [tagItem.meta.id, tagItem])), [tags]);
  const folderRows = useMemo(() => flattenNoteFolders(folders), [folders]);

  const [scope, setScope] = useState<NotesScope>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [properties, setProperties] = useState<NoteProperties>({ status: "", source: "", aliases: [] });
  const [frontmatterTags, setFrontmatterTags] = useState<string[]>([]);
  const [frontmatterFavorite, setFrontmatterFavorite] = useState<boolean | null>(null);
  const [editorSelection, setEditorSelection] = useState("");
  const [notice, setNotice] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [mobileEditing, setMobileEditing] = useState(false);
  const [cloudSavedNoteId, setCloudSavedNoteId] = useState<string | null>(null);
  const [cloudSaveRevision, setCloudSaveRevision] = useState(0);
  const [showNewFolder, setShowNewFolder] = useState(false);
  const [folderSettingsOpen, setFolderSettingsOpen] = useState(false);
  const [showNewTag, setShowNewTag] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
  const [newFolderParentId, setNewFolderParentId] = useState("");
  const [newTagName, setNewTagName] = useState("");
  const [commandOpen, setCommandOpen] = useState(false);
  const [graphOpen, setGraphOpen] = useState(false);
  const [revisionOpen, setRevisionOpen] = useState(false);
  const [openedIds, setOpenedIds] = useState<string[]>([]);
  const autosaveRef = useRef<number | null>(null);

  const noteTagIds = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const relation of tagRelations) {
      const noteId = text(relation, "noteId");
      const tagId = text(relation, "tagId");
      if (!noteId || !tagId) continue;
      const current = map.get(noteId) ?? new Set<string>();
      current.add(tagId);
      map.set(noteId, current);
    }
    return map;
  }, [tagRelations]);

  const scopedNotes = useMemo(() => {
    if (scope === "trash") return archivedNotes;

    let values = activeNotes;
    if (scope === "inbox") values = values.filter((note) => !text(note, "folderId"));
    if (scope === "recent") values = values.slice(0, 30);
    if (scope === "favorites") values = values.filter((note) => note.isFavorite === true);

    const folderId = scopeId(scope, "folder");
    if (folderId) values = values.filter((note) => text(note, "folderId") === folderId);

    const tagId = scopeId(scope, "tag");
    if (tagId) values = values.filter((note) => noteTagIds.get(note.meta.id)?.has(tagId));

    return values;
  }, [activeNotes, archivedNotes, noteTagIds, scope]);

  const visibleNotes = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase("zh-CN");
    if (!needle) return scopedNotes;

    return scopedNotes.filter((note) => {
      const folderName = text(folderById.get(text(note, "folderId")), "name");
      const tagNames = [...(noteTagIds.get(note.meta.id) ?? [])]
        .map((tagId) => text(tagById.get(tagId), "name"))
        .join(" ");
      return `${text(note, "title")} ${noteMarkdown(note)} ${folderName} ${tagNames}`
        .toLocaleLowerCase("zh-CN")
        .includes(needle);
    });
  }, [folderById, noteTagIds, query, scopedNotes, tagById]);

  const selected = notes.find((note) => note.meta.id === selectedId) ?? null;
  const selectedTagIds = selected ? noteTagIds.get(selected.meta.id) ?? new Set<string>() : new Set<string>();
  const inboxCount = activeNotes.filter((note) => !text(note, "folderId")).length;
  const favoriteCount = activeNotes.filter((note) => note.isFavorite === true).length;
  const activeFolderId = scopeId(scope, "folder");
  const activeFolder = activeFolderId ? folderById.get(activeFolderId) ?? null : null;

  useEffect(() => {
    if (!session?.user.id) {
      setOpenedIds([]);
      return;
    }
    try {
      const raw = localStorage.getItem(`lifetrace:notes:tabs:${session.user.id}`);
      const parsed = raw ? JSON.parse(raw) as string[] : [];
      setOpenedIds(Array.isArray(parsed) ? parsed.filter((id) => notes.some((note) => note.meta.id === id)).slice(0, 12) : []);
    } catch {
      setOpenedIds([]);
    }
  }, [session?.user.id]);

  useEffect(() => {
    if (!session?.user.id) return;
    try {
      localStorage.setItem(`lifetrace:notes:tabs:${session.user.id}`, JSON.stringify(openedIds.slice(0, 12)));
    } catch {
      // Tabs are convenience state only; Notes Cloud data remains authoritative.
    }
  }, [openedIds, session?.user.id]);

  useEffect(() => {
    const requestedId = searchParams.get("note");
    if (!requestedId || !notes.some((note) => note.meta.id === requestedId)) return;
    setSelectedId(requestedId);
    setOpenedIds((current) => [requestedId, ...current.filter((id) => id !== requestedId)].slice(0, 12));
    setMobileEditing(true);
    const next = new URLSearchParams(searchParams);
    next.delete("note");
    setSearchParams(next, { replace: true });
  }, [notes, searchParams, setSearchParams]);

  useEffect(() => {
    if (!selectedId) {
      const first = visibleNotes[0] ?? activeNotes[0] ?? archivedNotes[0] ?? null;
      if (first) setSelectedId(first.meta.id);
      return;
    }
    if (!selected) setSelectedId(visibleNotes[0]?.meta.id ?? activeNotes[0]?.meta.id ?? archivedNotes[0]?.meta.id ?? null);
  }, [activeNotes, archivedNotes, selected, selectedId, visibleNotes]);

  useEffect(() => {
    if (!selected) {
      setTitle("");
      setContent("");
      setProperties({ status: "", source: "", aliases: [] });
      setFrontmatterTags([]);
      setFrontmatterFavorite(null);
      setEditorSelection("");
      setSaveState("saved");
      return;
    }
    const parsed = parseFrontmatter(noteMarkdown(selected));
    const mergedProperties = mergeNoteProperties(
      storedNoteProperties(selected.contentJson),
      parsed.properties,
    );
    setTitle(parsed.properties.title?.trim() || text(selected, "title"));
    setContent(parsed.body);
    setProperties(mergedProperties);
    setFrontmatterTags(parsed.properties.tags);
    setFrontmatterFavorite(parsed.properties.favorite ?? null);
    setEditorSelection("");
    setSaveState(parsed.found ? "dirty" : "saved");
  }, [selected?.meta.id]);

  const syncWikiRelations = useCallback(async (noteId: string, markdown: string) => {
    if (!session) return;
    const desired = new Map<string, { targetId: string; linkText: string }>();
    for (const target of uniqueWikiTargets(markdown)) {
      const targetKey = target.trim().toLocaleLowerCase("zh-CN");
      const targetNote = activeNotes.find((candidate) => {
        if (candidate.meta.id === noteId) return false;
        if (text(candidate, "title").trim().toLocaleLowerCase("zh-CN") === targetKey) return true;
        return noteAliases(candidate.contentJson)
          .some((alias) => alias.toLocaleLowerCase("zh-CN") === targetKey);
      });
      if (targetNote) desired.set(targetNote.meta.id, { targetId: targetNote.meta.id, linkText: target });
    }

    const existing = noteRelations.filter((relation) =>
      text(relation, "noteId") === noteId
      && text(relation, "relationType", "wiki_link") === "wiki_link"
    );

    for (const relation of existing) {
      const targetId = text(relation, "entityId");
      if (!desired.has(targetId)) await remove("note.relation", relation.meta.id);
    }
    for (const value of desired.values()) {
      if (existing.some((relation) => text(relation, "entityId") === value.targetId)) continue;
      await upsert("note.relation", createNoteRelation(
        session.user.id,
        session.session.deviceId,
        noteId,
        value.targetId,
      ));
    }
  }, [activeNotes, noteRelations, remove, session, upsert]);

  const snapshotRevision = useCallback(async (note: JsonEntity) => {
    if (!session) return;
    const existing = revisions
      .filter((revision) => text(revision, "noteId") === note.meta.id)
      .sort((a, b) => number(b, "revisionVersion") - number(a, "revisionVersion"));
    const nextVersion = (existing[0] ? number(existing[0], "revisionVersion") : 0) + 1;
    await upsert(
      "note.revision",
      createNoteRevision(
        session.user.id,
        session.session.deviceId,
        note,
        nextVersion,
      ),
    );

    // Keep history bounded for the personal-cloud deployment. The just-created
    // revision plus the newest 49 existing snapshots gives at most 50 versions.
    for (const stale of existing.slice(49)) {
      await remove("note.revision", stale.meta.id);
    }
  }, [remove, revisions, session, upsert]);

  const save = useCallback(async () => {
    if (!session || !selected) return;
    if (autosaveRef.current !== null) window.clearTimeout(autosaveRef.current);
    setSaveState("saving");
    try {
      const favorite = frontmatterFavorite ?? selected.isFavorite === true;
      const nextNote = notePayload(selected, title, content, properties, favorite);
      await snapshotRevision(selected);
      await upsert("note.note", nextNote);

      for (const importedName of frontmatterTags) {
        const key = importedName.toLocaleLowerCase("zh-CN");
        let tagItem = tags.find((item) => text(item, "name").toLocaleLowerCase("zh-CN") === key);
        if (!tagItem) {
          tagItem = createNoteTag(session.user.id, session.session.deviceId, importedName);
          await upsert("note.tag", tagItem);
        }
        const exists = tagRelations.some((relation) =>
          text(relation, "noteId") === selected.meta.id && text(relation, "tagId") === tagItem!.meta.id
        );
        if (!exists) {
          await upsert("note.tag_relation", createNoteTagRelation(
            session.user.id,
            session.session.deviceId,
            selected.meta.id,
            tagItem.meta.id,
          ));
        }
      }

      await syncWikiRelations(selected.meta.id, content);
      setFrontmatterTags([]);
      setFrontmatterFavorite(null);
      setCloudSavedNoteId(selected.meta.id);
      setCloudSaveRevision((revision) => revision + 1);
      setSaveState("saved");
    } catch {
      setSaveState("error");
      throw new Error("笔记保存失败");
    }
  }, [content, frontmatterFavorite, frontmatterTags, properties, selected, session, snapshotRevision, syncWikiRelations, tagRelations, tags, title, upsert]);

  useEffect(() => {
    if (!selected || saveState !== "dirty") return;
    if (autosaveRef.current !== null) window.clearTimeout(autosaveRef.current);
    autosaveRef.current = window.setTimeout(() => { void save().catch(() => undefined); }, 800);
    return () => {
      if (autosaveRef.current !== null) window.clearTimeout(autosaveRef.current);
    };
  }, [content, properties, title, selected?.meta.id, saveState, save]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key.toLowerCase() === "s") {
        event.preventDefault();
        void save().catch(() => undefined);
      }
      if (event.key.toLowerCase() === "n") {
        event.preventDefault();
        void newNote();
      }
      if (event.key.toLowerCase() === "p") {
        event.preventDefault();
        setCommandOpen(true);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  async function changeScope(next: NotesScope) {
    if (next === scope) return;
    if (selected && saveState === "dirty") await save().catch(() => undefined);
    setScope(next);
    setSelectedId(null);
    setMobileEditing(false);
  }

  async function openDailyNote() {
    if (!session) return;
    if (selected && saveState === "dirty") await save().catch(() => undefined);
    const day = todayKey();
    let daily = activeNotes.find((note) =>
      text(note, "title") === day
      && storedNoteProperties(note.contentJson).status === "daily"
    ) ?? activeNotes.find((note) => text(note, "title") === day);

    if (!daily) {
      const markdown = [
        `# ${day}`,
        "",
        "## 今日记录",
        "",
        "## 待处理",
        "",
        "## 复盘",
        "",
      ].join("\n");
      daily = createNote(session.user.id, session.session.deviceId, day, markdown);
      daily = {
        ...daily,
        contentJson: {
          type: "markdown",
          source: markdown,
          editor: "vditor",
          properties: { status: "daily", source: "lifetrace", aliases: [] },
        },
      };
      await upsert("note.note", daily);
    }

    setScope("all");
    setSelectedId(daily.meta.id);
    setOpenedIds((current) => [daily!.meta.id, ...current.filter((id) => id !== daily!.meta.id)].slice(0, 12));
    setMobileEditing(true);
  }

  async function newNote() {
    if (!session) return;
    if (selected && saveState === "dirty") await save().catch(() => undefined);
    const activeFolderId = scopeId(scope, "folder");
    const activeTagId = scopeId(scope, "tag");
    const note = createNote(session.user.id, session.session.deviceId, "新笔记", "# 新笔记\n\n", activeFolderId);
    await upsert("note.note", note);
    if (activeTagId) {
      await upsert("note.tag_relation", createNoteTagRelation(session.user.id, session.session.deviceId, note.meta.id, activeTagId));
    }
    if (scope === "trash") setScope("all");
    setSelectedId(note.meta.id);
    setOpenedIds((current) => [note.meta.id, ...current.filter((id) => id !== note.meta.id)].slice(0, 12));
    setMobileEditing(true);
  }

  async function selectNote(id: string) {
    if (id === selectedId) { setMobileEditing(true); return; }
    if (selected && saveState === "dirty") await save().catch(() => undefined);
    setSelectedId(id);
    setOpenedIds((current) => [id, ...current.filter((item) => item !== id)].slice(0, 12));
    setMobileEditing(true);
  }

  function closeTab(id: string) {
    setOpenedIds((current) => current.filter((item) => item !== id));
    if (selectedId !== id) return;
    const remaining = openedIds.filter((item) => item !== id);
    setSelectedId(remaining[0] ?? visibleNotes.find((note) => note.meta.id !== id)?.meta.id ?? null);
  }

  function insertMarkdown(value: string) {
    if (!selected || selected.isArchived === true) return;
    setContent((current) => appendMarkdownLink(current, value));
    setSaveState("dirty");
  }

  async function createTaskFromNote() {
    if (!session || !selected) return;
    if (saveState === "dirty") await save().catch(() => undefined);
    const sourceText = editorSelection.trim() || content.trim();
    const plain = plainTextFromMarkdown(sourceText);
    const taskTitle = (editorSelection.trim() ? plain.split(/\r?\n/)[0] : title.trim()) || "处理笔记";
    const task = createExecutionTask(session.user.id, session.session.deviceId, {
      title: taskTitle.slice(0, 160),
      description: `${sourceText.slice(0, 4000)}\n\nSource: notes://note/${selected.meta.id}`,
      context: "inbox",
    });
    await upsert("execution.task", task);
    await upsert("entity.link", createEntityLink(
      session.user.id,
      session.session.deviceId,
      "note.note",
      selected.meta.id,
      "created_from",
      "execution.task",
      task.meta.id,
      { sourceUri: `notes://note/${selected.meta.id}`, selection: editorSelection.trim() || null },
    ));
    setNotice(editorSelection.trim() ? "已从选中文本创建 Task。" : "已从当前笔记创建 Task。");
  }

  function updateProperties(next: NoteProperties) {
    setProperties(next);
    setSaveState("dirty");
  }

  async function toggleFavorite() {
    if (!selected) return;
    await upsert("note.note", { ...selected, isFavorite: selected.isFavorite !== true });
  }

  async function archiveSelected() {
    if (!selected) return;
    if (saveState === "dirty") await save().catch(() => undefined);
    await upsert("note.note", { ...selected, isArchived: true });
    setSelectedId(null);
    setMobileEditing(false);
  }

  async function restoreSelected() {
    if (!selected) return;
    await upsert("note.note", { ...selected, isArchived: false });
    setScope("all");
    setSelectedId(selected.meta.id);
  }

  async function restoreRevision(revision: JsonEntity) {
    if (!session || !selected) return;

    const currentFavorite = frontmatterFavorite ?? selected.isFavorite === true;
    const currentSnapshot = notePayload(selected, title, content, properties, currentFavorite);
    await snapshotRevision(currentSnapshot);

    const markdown = text(revision, "contentMarkdown");
    const restoredTitle = typeof revision.title === "string" ? revision.title : "";
    const restoredJson = revision.contentJson && typeof revision.contentJson === "object"
      ? revision.contentJson
      : {};
    const plainText = plainTextFromMarkdown(markdown);
    const restored = {
      ...selected,
      title: restoredTitle.trim() || null,
      contentJson: restoredJson,
      contentHtml: text(revision, "contentHtml"),
      contentMarkdown: markdown,
      contentText: plainText,
      summary: plainText.slice(0, 160),
    };

    await upsert("note.note", restored);
    await syncWikiRelations(selected.meta.id, markdown);

    setTitle(restoredTitle);
    setContent(markdown);
    setProperties(storedNoteProperties(restoredJson));
    setFrontmatterTags([]);
    setFrontmatterFavorite(null);
    setCloudSavedNoteId(selected.meta.id);
    setCloudSaveRevision((value) => value + 1);
    setSaveState("saved");
    setRevisionOpen(false);
    setNotice("已恢复历史版本；恢复前的当前内容也已自动保留为一个新版本。");
  }

  async function permanentlyDeleteSelected() {
    if (!selected) return;
    const relatedTags = tagRelations.filter((relation) => text(relation, "noteId") === selected.meta.id);
    const relatedLinks = noteRelations.filter((relation) =>
      text(relation, "noteId") === selected.meta.id || (text(relation, "entityType") === "note.note" && text(relation, "entityId") === selected.meta.id)
    );
    const relatedRevisions = revisions.filter((revision) => text(revision, "noteId") === selected.meta.id);
    for (const relation of relatedTags) await remove("note.tag_relation", relation.meta.id);
    for (const relation of relatedLinks) await remove("note.relation", relation.meta.id);
    for (const revision of relatedRevisions) await remove("note.revision", revision.meta.id);
    await remove("note.note", selected.meta.id);
    setSelectedId(null);
    setMobileEditing(false);
  }

  async function moveSelected(folderId: string) {
    if (!selected) return;
    if (saveState === "dirty") await save().catch(() => undefined);
    await upsert("note.note", { ...selected, folderId: folderId || null });
  }

  async function addSelectedTag(tagId: string) {
    if (!session || !selected || !tagId || selectedTagIds.has(tagId)) return;
    await upsert("note.tag_relation", createNoteTagRelation(session.user.id, session.session.deviceId, selected.meta.id, tagId));
  }

  async function removeSelectedTag(tagId: string) {
    if (!selected) return;
    const relation = tagRelations.find((item) => text(item, "noteId") === selected.meta.id && text(item, "tagId") === tagId);
    if (relation) await remove("note.tag_relation", relation.meta.id);
  }

  async function submitFolder(event: FormEvent) {
    event.preventDefault();
    if (!session || !newFolderName.trim()) return;
    const folder = createNoteFolder(session.user.id, session.session.deviceId, newFolderName, folders.length, newFolderParentId || null);
    await upsert("note.folder", folder);
    setNewFolderName("");
    setNewFolderParentId("");
    setShowNewFolder(false);
    await changeScope(`folder:${folder.meta.id}`);
  }

  async function saveActiveFolder(name: string, parentFolderId: string | null) {
    if (!activeFolder) return;
    await upsert("note.folder", {
      ...activeFolder,
      name,
      parentFolderId,
    });
  }

  async function deleteActiveFolder() {
    if (!activeFolder) return;
    const parentFolderId = text(activeFolder, "parentFolderId") || null;
    const childFolders = folders.filter((folder) => text(folder, "parentFolderId") === activeFolder.meta.id);
    const childNotes = activeNotes.filter((note) => text(note, "folderId") === activeFolder.meta.id);

    for (const child of childFolders) {
      await upsert("note.folder", { ...child, parentFolderId });
    }
    for (const note of childNotes) {
      await upsert("note.note", { ...note, folderId: parentFolderId });
    }
    await remove("note.folder", activeFolder.meta.id);
    setScope(parentFolderId ? `folder:${parentFolderId}` : "all");
    setSelectedId(null);
    setMobileEditing(false);
  }

  async function submitTag(event: FormEvent) {
    event.preventDefault();
    if (!session || !newTagName.trim()) return;
    const existing = tags.find((item) => text(item, "name").toLocaleLowerCase("zh-CN") === newTagName.trim().toLocaleLowerCase("zh-CN"));
    if (existing) {
      setNewTagName("");
      setShowNewTag(false);
      await changeScope(`tag:${existing.meta.id}`);
      return;
    }
    const tagItem = createNoteTag(session.user.id, session.session.deviceId, newTagName);
    await upsert("note.tag", tagItem);
    setNewTagName("");
    setShowNewTag(false);
    await changeScope(`tag:${tagItem.meta.id}`);
  }

  const saveLabel = saveState === "saving" ? "保存中" : saveState === "dirty" ? "未保存" : saveState === "error" ? "保存失败" : "已保存";
  const builtinViews = [
    { id: "inbox" as const, label: "Inbox", icon: Inbox, count: inboxCount },
    { id: "all" as const, label: "全部笔记", icon: NotebookPen, count: activeNotes.length },
    { id: "recent" as const, label: "最近", icon: Clock3, count: Math.min(activeNotes.length, 30) },
    { id: "favorites" as const, label: "收藏", icon: Star, count: favoriteCount },
    { id: "trash" as const, label: "废纸篓", icon: Trash2, count: archivedNotes.length },
  ];

  const activeScopeLabel =
    builtinViews.find((item) => item.id === scope)?.label
    || text(folderById.get(scopeId(scope, "folder") ?? ""), "name")
    || (scopeId(scope, "tag") ? `#${text(tagById.get(scopeId(scope, "tag") ?? ""), "name")}` : "")
    || "全部笔记";

  return <WorkspaceShell
    title="Notes"
    description="Markdown 知识库与信息组织中心"
    icon={<NotebookPen size={17} />}
    action={<>
      <Button size="sm" variant="outline" onClick={() => void openDailyNote()}><CalendarDays size={14} /><span className="hidden lg:inline">Daily</span></Button>
      <Button size="sm" variant="outline" onClick={() => setGraphOpen(true)}><Network size={14} /><span className="hidden lg:inline">Graph</span></Button>
      <Button size="sm" variant="outline" onClick={() => setCommandOpen(true)}><Command size={14} /><span className="hidden sm:inline">命令</span></Button>
      <Button size="sm" onClick={() => void newNote()}><Plus size={15} /><span className="hidden sm:inline">新建</span></Button>
    </>}
  >
    {notice ? <div className="mx-3 mt-3 flex items-center justify-between gap-3 rounded-md border bg-card px-3 py-2 text-xs sm:mx-5"><span>{notice}</span><button className="text-muted-foreground hover:text-foreground" onClick={() => setNotice("")}>关闭</button></div> : null}
    <div className="grid min-h-[calc(100vh-6rem)] lg:min-h-[calc(100vh-4rem)] lg:grid-cols-[320px_minmax(0,1fr)] xl:grid-cols-[224px_320px_minmax(0,1fr)]">
      <aside className="hidden border-r bg-card/45 p-3 xl:block">
        <div className="mb-4 px-2 pt-1">
          <div className="text-xs font-semibold">知识库</div>
          <div className="mt-0.5 text-[11px] text-muted-foreground">LifeTrace Notes</div>
        </div>

        <nav className="space-y-1" aria-label="笔记导航">
          {builtinViews.map(({ id, label, icon: Icon, count }) => <button
            key={id}
            onClick={() => void changeScope(id)}
            className={cn("flex h-9 w-full items-center gap-2 rounded-md px-2.5 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground", scope === id && "bg-accent font-medium text-accent-foreground")}
          ><Icon size={16} /><span className="flex-1 text-left">{label}</span><span className="text-[11px]">{count}</span></button>)}
        </nav>

        <div className="mt-5 border-t pt-4">
          <div className="mb-1 flex items-center justify-between px-2">
            <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Folders</div>
            <div className="flex items-center gap-0.5">
              {activeFolder ? <button className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground" onClick={() => setFolderSettingsOpen(true)} aria-label="文件夹设置"><Settings2 size={14} /></button> : null}
              <button className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground" onClick={() => { setNewFolderParentId(scopeId(scope, "folder") ?? ""); setShowNewFolder((value) => !value); }} aria-label="新建文件夹"><FolderPlus size={14} /></button>
            </div>
          </div>
          {showNewFolder ? <form className="mb-2 space-y-1.5" onSubmit={(event) => void submitFolder(event)}>
            <div className="flex gap-1">
              <Input autoFocus className="h-8 px-2 text-xs" value={newFolderName} onChange={(event) => setNewFolderName(event.target.value)} placeholder="文件夹名称" />
              <Button size="icon" className="h-8 w-8" type="submit" aria-label="创建文件夹"><Check size={14} /></Button>
            </div>
            <select className="h-8 w-full rounded-md border bg-background px-2 text-xs text-muted-foreground" value={newFolderParentId} onChange={(event) => setNewFolderParentId(event.target.value)} aria-label="父文件夹">
              <option value="">根目录</option>
              {folderRows.map(({ folder, depth, path }) => <option key={folder.meta.id} value={folder.meta.id}>{`${"— ".repeat(depth)}${path}`}</option>)}
            </select>
          </form> : null}
          <div className="space-y-1">
            {folderRows.map(({ folder, depth }) => {
              const id = folder.meta.id;
              const count = activeNotes.filter((note) => text(note, "folderId") === id).length;
              return <button
                key={id}
                onClick={() => void changeScope(`folder:${id}`)}
                style={{ paddingLeft: `${10 + depth * 14}px` }}
                className={cn("flex h-8 w-full items-center gap-2 rounded-md pr-2.5 text-xs text-muted-foreground hover:bg-muted hover:text-foreground", scope === `folder:${id}` && "bg-accent font-medium text-accent-foreground")}
              ><Folder size={14} className="shrink-0" /><span className="flex-1 truncate text-left">{text(folder, "name", "未命名")}</span><span className="text-[10px]">{count}</span></button>;
            })}
            {!folders.length && !showNewFolder ? <div className="px-2 py-1 text-[11px] text-muted-foreground">还没有文件夹</div> : null}
          </div>
        </div>

        <div className="mt-4 border-t pt-4">
          <div className="mb-1 flex items-center justify-between px-2">
            <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">Tags</div>
            <button className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground" onClick={() => setShowNewTag((value) => !value)} aria-label="新建标签"><Plus size={14} /></button>
          </div>
          {showNewTag ? <form className="mb-2 flex gap-1" onSubmit={(event) => void submitTag(event)}>
            <Input autoFocus className="h-8 px-2 text-xs" value={newTagName} onChange={(event) => setNewTagName(event.target.value)} placeholder="标签名称" />
            <Button size="icon" className="h-8 w-8" type="submit" aria-label="创建标签"><Check size={14} /></Button>
          </form> : null}
          <div className="space-y-1">
            {tags.map((tagItem) => {
              const id = tagItem.meta.id;
              const count = activeNotes.filter((note) => noteTagIds.get(note.meta.id)?.has(id)).length;
              return <button key={id} onClick={() => void changeScope(`tag:${id}`)} className={cn("flex h-8 w-full items-center gap-2 rounded-md px-2.5 text-xs text-muted-foreground hover:bg-muted hover:text-foreground", scope === `tag:${id}` && "bg-accent font-medium text-accent-foreground")}><Tag size={14} /><span className="flex-1 truncate text-left">{text(tagItem, "name", "未命名")}</span><span className="text-[10px]">{count}</span></button>;
            })}
            {!tags.length && !showNewTag ? <div className="px-2 py-1 text-[11px] text-muted-foreground">还没有标签</div> : null}
          </div>
        </div>
      </aside>

      <section className={cn("min-w-0 border-b lg:block lg:border-b-0 lg:border-r", mobileEditing && "hidden lg:block")}>
        <div className="border-b p-3">
          <div className="mb-2 flex items-center justify-between gap-2 xl:hidden">
            <select className="h-8 max-w-[68%] rounded-md border bg-background px-2 text-xs" value={scope} onChange={(event) => void changeScope(event.target.value as NotesScope)} aria-label="笔记范围">
              <optgroup label="笔记">
                {builtinViews.map((item) => <option key={item.id} value={item.id}>{item.label} ({item.count})</option>)}
              </optgroup>
              {folderRows.length ? <optgroup label="文件夹">{folderRows.map(({ folder, depth }) => <option key={folder.meta.id} value={`folder:${folder.meta.id}`}>{`${"— ".repeat(depth)}${text(folder, "name")}`}</option>)}</optgroup> : null}
              {tags.length ? <optgroup label="标签">{tags.map((tagItem) => <option key={tagItem.meta.id} value={`tag:${tagItem.meta.id}`}>#{text(tagItem, "name")}</option>)}</optgroup> : null}
            </select>
            <Button size="sm" onClick={() => void newNote()}><Plus size={14} />新建</Button>
          </div>
          <div className="mb-2 flex items-center justify-between gap-2">
            <div className="min-w-0 flex-1 truncate text-xs font-medium">{activeScopeLabel}</div>
            {activeFolder ? <Button className="xl:hidden" size="icon" variant="ghost" aria-label="文件夹设置" onClick={() => setFolderSettingsOpen(true)}><Settings2 size={13} /></Button> : null}
            <span className="text-[11px] text-muted-foreground">{visibleNotes.length} 篇</span>
          </div>
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" size={15} />
            <Input className="h-9 border-0 bg-muted/55 pl-9 focus:ring-0" placeholder="搜索标题、正文、文件夹或标签" value={query} onChange={(event) => setQuery(event.target.value)} />
          </div>
        </div>
        <div className="scrollbar-thin max-h-[calc(100vh-12rem)] overflow-y-auto p-2 lg:max-h-[calc(100vh-10rem)]">
          {visibleNotes.length ? visibleNotes.map((note) => <button key={note.meta.id} onClick={() => void selectNote(note.meta.id)} className={cn("mb-1 w-full rounded-md px-3 py-2.5 text-left", selectedId === note.meta.id ? "bg-accent" : "hover:bg-muted")}>
            <div className="flex items-center gap-2">
              <div className="min-w-0 flex-1 truncate text-sm font-medium">{text(note, "title", "无标题")}</div>
              {note.isFavorite === true ? <Star size={12} className="shrink-0 fill-current text-warning" /> : null}
            </div>
            <div className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{text(note, "summary", markdownSummary(noteMarkdown(note)) || "空笔记")}</div>
            <div className="mt-1 flex items-center gap-2 text-[11px] text-muted-foreground">
              <span>{new Date(note.meta.updatedAt).toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}</span>
              {text(note, "folderId") ? <span className="truncate">· {text(folderById.get(text(note, "folderId")), "name")}</span> : null}
            </div>
          </button>) : <EmptyState title={query ? "没有匹配的笔记" : "这里还没有笔记"} description={query ? "搜索会同时匹配标题、Markdown 正文、文件夹和标签。" : scope === "trash" ? "废纸篓是空的。" : "创建一篇 Markdown 笔记开始记录。"} />}
        </div>
      </section>

      <main className={cn("min-w-0 bg-background p-3 sm:p-5", !mobileEditing && "hidden lg:block")}>
        {selected ? <>
          <NoteTabs notes={notes} openedIds={openedIds} activeId={selectedId} onSelect={(id) => void selectNote(id)} onClose={closeTab} />
          <div className="mb-3 flex items-center gap-2">
            <Button className="lg:hidden" variant="ghost" size="icon" aria-label="返回笔记列表" onClick={() => setMobileEditing(false)}><ArrowLeft size={17} /></Button>
            <Input className="h-auto min-w-0 flex-1 border-0 px-0 text-xl font-semibold shadow-none focus:ring-0" placeholder="无标题" value={title} disabled={selected.isArchived === true} onChange={(event) => { setTitle(event.target.value); setSaveState("dirty"); }} />
            <span className={cn("hidden items-center gap-1 text-xs sm:flex", saveState === "error" ? "text-destructive" : "text-muted-foreground")}>{saveState === "saving" ? <Loader2 size={13} className="animate-spin" /> : saveState === "saved" ? <Check size={13} /> : null}{selected.isArchived === true ? "废纸篓" : saveLabel}</span>
            {selected.isArchived === true ? <>
              <Button variant="outline" size="icon" aria-label="恢复笔记" onClick={() => void restoreSelected()}><RotateCcw size={15} /></Button>
              <Button variant="destructive" size="icon" aria-label="永久删除笔记" onClick={() => void permanentlyDeleteSelected()}><Trash2 size={15} /></Button>
            </> : <>
              <Button variant="ghost" size="icon" aria-label={selected.isFavorite === true ? "取消收藏" : "收藏笔记"} onClick={() => void toggleFavorite()}><Star size={16} className={selected.isFavorite === true ? "fill-current text-warning" : ""} /></Button>
              <Button variant="ghost" size="icon" aria-label="从笔记创建任务" title={editorSelection.trim() ? "从选中文本创建 Task" : "从当前笔记创建 Task"} onClick={() => void createTaskFromNote()}><CheckSquare2 size={16} /></Button>
              <Button variant="ghost" size="icon" aria-label="版本历史" title="版本历史" onClick={() => setRevisionOpen(true)}><History size={15} /></Button>
              <Button variant="outline" size="icon" aria-label="保存笔记" onClick={() => void save().catch(() => undefined)}><Save size={15} /></Button>
              <Button variant="ghost" size="icon" aria-label="移到废纸篓" onClick={() => void archiveSelected()}><Trash2 size={15} /></Button>
            </>}
          </div>

          {selected.isArchived !== true ? <div className="mb-3 flex flex-wrap items-center gap-2 border-y py-2">
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground"><Folder size={14} />文件夹</div>
            <select className="h-8 rounded-md border bg-background px-2 text-xs" value={text(selected, "folderId")} onChange={(event) => void moveSelected(event.target.value)} aria-label="移动笔记到文件夹">
              <option value="">Inbox</option>
              {folderRows.map(({ folder, depth }) => <option key={folder.meta.id} value={folder.meta.id}>{`${"— ".repeat(depth)}${text(folder, "name")}`}</option>)}
            </select>
            <div className="ml-1 flex items-center gap-1.5 text-xs text-muted-foreground"><Tag size={14} />标签</div>
            {[...selectedTagIds].map((tagId) => {
              const tagItem = tagById.get(tagId);
              if (!tagItem) return null;
              return <span key={tagId} className="inline-flex h-7 items-center gap-1 rounded-md border bg-muted/40 px-2 text-xs">#{text(tagItem, "name")}<button aria-label={`移除标签 ${text(tagItem, "name")}`} onClick={() => void removeSelectedTag(tagId)} className="text-muted-foreground hover:text-foreground"><X size={12} /></button></span>;
            })}
            {tags.some((tagItem) => !selectedTagIds.has(tagItem.meta.id)) ? <select className="h-8 rounded-md border bg-background px-2 text-xs text-muted-foreground" value="" onChange={(event) => void addSelectedTag(event.target.value)} aria-label="添加标签">
              <option value="">+ 添加标签</option>
              {tags.filter((tagItem) => !selectedTagIds.has(tagItem.meta.id)).map((tagItem) => <option key={tagItem.meta.id} value={tagItem.meta.id}>#{text(tagItem, "name")}</option>)}
            </select> : null}
          </div> : null}

          <div className="grid gap-4 2xl:grid-cols-[minmax(0,1fr)_260px]">
            <div className={selected.isArchived === true ? "pointer-events-none opacity-80" : undefined}>
              <VditorEditor
                key={selected.meta.id}
                value={content}
                cacheKey={`lifetrace:vditor:${session?.user.id ?? "anonymous"}:${selected.meta.id}`}
                cloudSaveRevision={cloudSavedNoteId === selected.meta.id ? cloudSaveRevision : 0}
                wikiSuggestions={activeNotes
                  .filter((note) => note.meta.id !== selected.meta.id)
                  .map((note) => ({ title: text(note, "title", "无标题"), aliases: noteAliases(note.contentJson) }))}
                onSelectionChange={setEditorSelection}
                onChange={(next) => { if (selected.isArchived !== true) { setContent(next); setSaveState("dirty"); } }}
                onSave={() => { if (selected.isArchived !== true) void save().catch(() => undefined); }}
              />
            </div>
            <div className="space-y-4">
              <NotePropertiesPanel
                note={selected}
                title={title}
                properties={properties}
                tagNames={[...selectedTagIds].map((tagId) => text(tagById.get(tagId), "name")).filter(Boolean)}
                favorite={(frontmatterFavorite ?? selected.isFavorite === true)}
                disabled={selected.isArchived === true}
                onChange={updateProperties}
              />
              <NotesKnowledgePanel note={selected} notes={activeNotes} relations={noteRelations} onOpenNote={(id) => void selectNote(id)} />
              {selected.isArchived !== true ? <NoteAttachments noteId={selected.meta.id} onInsertMarkdown={insertMarkdown} /> : null}
            </div>
          </div>
        </> : <EmptyState title="选择一篇笔记" description="从列表选择，或创建新的 Markdown 笔记。" />}
      </main>
    </div>
    <Dialog open={graphOpen} onOpenChange={setGraphOpen} title="Notes Graph" description="基于已同步的 Wiki Link 关系生成轻量知识图谱。点击节点打开对应笔记。">
      <NotesGraphView notes={activeNotes} relations={noteRelations} onOpenNote={(id) => { setGraphOpen(false); void selectNote(id); }} />
    </Dialog>
    <NoteRevisionHistory
      open={revisionOpen}
      onOpenChange={setRevisionOpen}
      noteId={selectedId}
      revisions={revisions}
      onRestore={restoreRevision}
    />
    <NoteFolderSettings
      open={folderSettingsOpen}
      onOpenChange={setFolderSettingsOpen}
      folder={activeFolder}
      folders={folders}
      onSave={saveActiveFolder}
      onDelete={deleteActiveFolder}
    />
    <NotesCommandPalette
      open={commandOpen}
      onOpenChange={setCommandOpen}
      notes={activeNotes}
      folders={folders}
      tags={tags}
      selectedId={selectedId}
      selectedTagIds={selectedTagIds}
      onNewNote={() => void newNote()}
      onOpenNote={(id) => void selectNote(id)}
      onScope={(next) => void changeScope(next)}
      onMoveSelected={(folderId) => void moveSelected(folderId)}
      onArchiveSelected={() => void archiveSelected()}
      onAddSelectedTag={(tagId) => void addSelectedTag(tagId)}
    />
  </WorkspaceShell>;
}
