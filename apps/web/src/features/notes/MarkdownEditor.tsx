import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState, type ReactNode } from "react";
import { autocompletion, type CompletionContext } from "@codemirror/autocomplete";
import { EditorView } from "@codemirror/view";
import { CodeMirrorMarkdownEditor, type MarkdownExtension } from "@latentic/live-markdown";
import "@latentic/live-markdown/styles.css";
import "katex/dist/katex.min.css";
import {
  Bold, Code2, Heading2, Italic, Link2, List, ListChecks, ListOrdered, Minus,
  Quote, Strikethrough, Table2,
} from "lucide-react";
import { API_BASE } from "../../services/core";

type Draft = {
  value: string;
  dirty: boolean;
  updatedAt: string;
};

export interface WikiSuggestion {
  title: string;
  aliases?: string[];
}

export interface MarkdownEditorHandle {
  focusLine(lineNumber: number): void;
}

export interface MarkdownEditorProps {
  value: string;
  cacheKey: string;
  legacyCacheKey?: string;
  cloudSaveRevision: number;
  wikiSuggestions?: WikiSuggestion[];
  onChange(value: string): void;
  onSave?(): void;
  onSelectionChange?(value: string): void;
}

function draftKey(cacheKey: string) {
  return cacheKey + ":markdown";
}

function readDraft(cacheKey: string, legacyCacheKey?: string): Draft | null {
  try {
    const raw = localStorage.getItem(draftKey(cacheKey));
    if (raw) return JSON.parse(raw) as Draft;

    if (!legacyCacheKey) return null;
    const legacyMetaRaw = localStorage.getItem(legacyCacheKey + ":meta");
    const legacyMeta = legacyMetaRaw
      ? JSON.parse(legacyMetaRaw) as { dirty?: boolean; updatedAt?: string }
      : null;
    const legacyValue = localStorage.getItem(legacyCacheKey);
    if (!legacyMeta?.dirty || legacyValue === null) return null;

    return {
      value: legacyValue,
      dirty: true,
      updatedAt: legacyMeta.updatedAt ?? new Date().toISOString(),
    };
  } catch {
    return null;
  }
}

function clearLegacyDraft(legacyCacheKey?: string) {
  if (!legacyCacheKey) return;
  try {
    localStorage.removeItem(legacyCacheKey);
    localStorage.removeItem(legacyCacheKey + ":meta");
  } catch {
    // Ignore browser storage failures.
  }
}

function writeDraft(cacheKey: string, value: string, dirty: boolean) {
  try {
    localStorage.setItem(draftKey(cacheKey), JSON.stringify({
      value,
      dirty,
      updatedAt: new Date().toISOString(),
    } satisfies Draft));
  } catch {
    // Cloud autosave remains authoritative when browser storage is unavailable.
  }
}

function attachmentId(value: string): string | null {
  if (!value.startsWith("attachment://")) return null;
  const id = value.slice("attachment://".length).split(/[?#]/, 1)[0];
  return /^[0-9a-fA-F-]{36}$/.test(id) ? id : null;
}

function attachmentContentUrl(id: string): string {
  return API_BASE + "/api/v1/files/" + encodeURIComponent(id) + "/content";
}

function openMarkdownUrl(value: string) {
  const id = attachmentId(value);
  if (id) {
    window.open(attachmentContentUrl(id), "_blank", "noopener,noreferrer");
    return;
  }

  try {
    const url = new URL(value, window.location.origin);
    if (!["http:", "https:", "mailto:"].includes(url.protocol)) return;
    window.open(url.toString(), "_blank", "noopener,noreferrer");
  } catch {
    // Ignore malformed links instead of handing them to the browser.
  }
}

function wrapSelection(view: EditorView, prefix: string, suffix = prefix, placeholder = "文本") {
  const selection = view.state.selection.main;
  const selected = selection.empty
    ? placeholder
    : view.state.doc.sliceString(selection.from, selection.to);
  const insert = prefix + selected + suffix;
  const selectedFrom = selection.from + prefix.length;

  view.dispatch({
    changes: { from: selection.from, to: selection.to, insert },
    selection: {
      anchor: selectedFrom,
      head: selectedFrom + selected.length,
    },
  });
  view.focus();
}

function prefixLines(view: EditorView, prefix: string) {
  const selection = view.state.selection.main;
  const start = view.state.doc.lineAt(selection.from).from;
  const end = view.state.doc.lineAt(selection.to).to;
  const source = view.state.doc.sliceString(start, end);
  const insert = source.split("\n").map((line) => prefix + line).join("\n");

  view.dispatch({
    changes: { from: start, to: end, insert },
    selection: { anchor: start + prefix.length, head: start + insert.length },
  });
  view.focus();
}

function insertSnippet(view: EditorView, snippet: string) {
  const selection = view.state.selection.main;
  view.dispatch({
    changes: { from: selection.from, to: selection.to, insert: snippet },
    selection: { anchor: selection.from + snippet.length },
  });
  view.focus();
}

function ToolButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick(): void;
  children: ReactNode;
}) {
  return <button
    type="button"
    aria-label={label}
    title={label}
    onMouseDown={(event) => event.preventDefault()}
    onClick={onClick}
    className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
  >
    {children}
  </button>;
}

export const MarkdownEditor = forwardRef<MarkdownEditorHandle, MarkdownEditorProps>(function MarkdownEditor({
  value,
  cacheKey,
  legacyCacheKey,
  cloudSaveRevision,
  wikiSuggestions = [],
  onChange,
  onSave,
  onSelectionChange,
}, ref) {
  const editorRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const onSaveRef = useRef(onSave);
  const onSelectionChangeRef = useRef(onSelectionChange);
  const wikiSuggestionsRef = useRef(wikiSuggestions);
  const initialDraftRef = useRef<Draft | null>(readDraft(cacheKey, legacyCacheKey));
  const initialValue = initialDraftRef.current?.dirty ? initialDraftRef.current.value : value;
  const liveValueRef = useRef(initialValue);
  const [editorValue, setEditorValue] = useState(initialValue);
  const externalSyncMountedRef = useRef(false);

  onChangeRef.current = onChange;
  onSaveRef.current = onSave;
  onSelectionChangeRef.current = onSelectionChange;
  wikiSuggestionsRef.current = wikiSuggestions;

  useImperativeHandle(ref, () => ({
    focusLine(lineNumber: number) {
      const editor = editorRef.current;
      if (!editor) return;
      const safeLine = Math.max(1, Math.min(lineNumber, editor.state.doc.lines));
      const line = editor.state.doc.line(safeLine);
      editor.dispatch({
        selection: { anchor: line.from },
        effects: EditorView.scrollIntoView(line.from, { y: "center" }),
      });
      editor.focus();
    },
  }), []);

  useEffect(() => {
    const cached = initialDraftRef.current;
    initialDraftRef.current = null;
    if (!cached?.dirty || cached.value === value) return;
    liveValueRef.current = cached.value;
    onChangeRef.current(cached.value);
  }, [value]);

  useEffect(() => {
    if (!externalSyncMountedRef.current) {
      externalSyncMountedRef.current = true;
      return;
    }
    if (value === liveValueRef.current) return;
    liveValueRef.current = value;
    setEditorValue(value);
  }, [value]);

  useEffect(() => {
    if (!cloudSaveRevision) return;
    writeDraft(cacheKey, liveValueRef.current, false);
    clearLegacyDraft(legacyCacheKey);
  }, [cacheKey, cloudSaveRevision, legacyCacheKey]);

  const hostExtension = useMemo<MarkdownExtension>(() => {
    function wikiCompletion(context: CompletionContext) {
      const match = context.matchBefore(/\[\[[^\]\n]*/);
      if (!match) return null;

      const needle = match.text.slice(2).trim().toLocaleLowerCase("zh-CN");
      const options = wikiSuggestionsRef.current
        .filter((item) => {
          if (!needle) return true;
          return [item.title, ...(item.aliases ?? [])]
            .some((candidate) => candidate.toLocaleLowerCase("zh-CN").includes(needle));
        })
        .slice(0, 12)
        .map((item) => ({
          label: item.title,
          detail: item.aliases?.length ? item.aliases.join(" · ") : undefined,
          apply: item.title + "]]",
          type: "text",
        }));

      return {
        from: match.from + 2,
        options,
        validFor: /^[^\]\n]*$/,
      };
    }

    return {
      name: "lifetrace-live-preview-bridge",
      version: "1.0.0",
      description: "Keep LifeTrace draft, autosave and selection state synchronized with rich Markdown rendering.",
      extensions: [
        autocompletion({
          activateOnTyping: true,
          override: [wikiCompletion],
        }),
        EditorView.theme({
          "&": {
            minHeight: "520px",
            backgroundColor: "hsl(var(--background))",
            color: "hsl(var(--foreground))",
            fontSize: "14px",
          },
          ".cm-scroller": {
            minHeight: "520px",
            fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", \"Microsoft YaHei\", sans-serif",
            lineHeight: "1.75",
          },
          ".cm-content": {
            padding: "18px 22px 120px",
            caretColor: "hsl(var(--foreground))",
          },
          ".cm-gutters": {
            display: "none",
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            const next = update.state.doc.toString();
            if (next !== liveValueRef.current) {
              liveValueRef.current = next;
              writeDraft(cacheKey, next, true);
              onChangeRef.current(next);
            }
          }

          if (update.selectionSet || update.docChanged) {
            const selection = update.state.selection.main;
            onSelectionChangeRef.current?.(
              selection.empty ? "" : update.state.doc.sliceString(selection.from, selection.to),
            );
          }
        }),
        EditorView.domEventHandlers({
          keydown(event) {
            if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
              event.preventDefault();
              onSaveRef.current?.();
              return true;
            }
            return false;
          },
        }),
      ],
    };
  }, [cacheKey]);

  const extensionModules = useMemo(() => [hostExtension] as const, [hostExtension]);

  const renderToolbar = useCallback(({ view }: { view: EditorView }) => {
    editorRef.current = view;
    return <div className="scrollbar-thin flex items-center gap-0.5 overflow-x-auto border-b bg-muted/20 px-2 py-1">
      <ToolButton label="二级标题" onClick={() => prefixLines(view, "## ")}><Heading2 size={15} /></ToolButton>
      <ToolButton label="粗体" onClick={() => wrapSelection(view, "**")}><Bold size={15} /></ToolButton>
      <ToolButton label="斜体" onClick={() => wrapSelection(view, "_")}><Italic size={15} /></ToolButton>
      <ToolButton label="删除线" onClick={() => wrapSelection(view, "~~")}><Strikethrough size={15} /></ToolButton>
      <ToolButton label="链接" onClick={() => wrapSelection(view, "[", "](https://)", "链接文字")}><Link2 size={15} /></ToolButton>
      <span className="mx-1 h-4 w-px shrink-0 bg-border" />
      <ToolButton label="无序列表" onClick={() => prefixLines(view, "- ")}><List size={15} /></ToolButton>
      <ToolButton label="有序列表" onClick={() => prefixLines(view, "1. ")}><ListOrdered size={15} /></ToolButton>
      <ToolButton label="任务列表" onClick={() => prefixLines(view, "- [ ] ")}><ListChecks size={15} /></ToolButton>
      <ToolButton label="引用" onClick={() => prefixLines(view, "> ")}><Quote size={15} /></ToolButton>
      <span className="mx-1 h-4 w-px shrink-0 bg-border" />
      <ToolButton label="行内代码" onClick={() => wrapSelection(view, String.fromCharCode(96))}><Code2 size={15} /></ToolButton>
      <ToolButton label="分隔线" onClick={() => insertSnippet(view, "\n---\n")}><Minus size={15} /></ToolButton>
      <ToolButton
        label="表格"
        onClick={() => insertSnippet(view, "\n| 列 1 | 列 2 |\n| --- | --- |\n|  |  |\n")}
      ><Table2 size={15} /></ToolButton>
      <div className="ml-auto hidden shrink-0 px-2 text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground sm:block">Live Markdown</div>
    </div>;
  }, []);

  const handleLibraryChange = useCallback((next: string) => {
    if (next === liveValueRef.current) return;
    liveValueRef.current = next;
    writeDraft(cacheKey, next, true);
    onChangeRef.current(next);
  }, [cacheKey]);

  const linkTargets = useMemo(
    () => new Set(wikiSuggestions.flatMap((item) => [item.title, ...(item.aliases ?? [])])),
    [wikiSuggestions],
  );

  return <div
    className="lifetrace-markdown-theme min-w-0 overflow-hidden rounded-md border bg-background text-foreground [&_.cm-editor-host]:min-h-[520px] [&_.cm-editor-host]:bg-background [&_.cm-editor-host__scroll]:min-h-[520px]"
    data-testid="markdown-editor"
    data-live-preview="true"
    data-render-engine="full"
    onMouseDownCapture={(event) => {
      const target = event.target as Element;
      if (!target.closest(".cm-table-inserter")) return;
      // The inserter's click listener belongs to live-markdown. Only keep
      // CodeMirror's selection/focus machinery from consuming the preceding
      // mouse-down and rebuilding the widget before click can fire.
      event.preventDefault();
      event.stopPropagation();
    }}
  >
    <CodeMirrorMarkdownEditor
      value={editorValue}
      onChange={handleLibraryChange}
      mode="wysiwyg"
      extensions={extensionModules}
      toolbar={renderToolbar}
      linkTargets={linkTargets}
      resolveImageSrc={(rawSrc) => {
        const id = attachmentId(rawSrc);
        return id ? attachmentContentUrl(id) : rawSrc;
      }}
      onOpenExternalUrl={openMarkdownUrl}
    />
  </div>;
});
