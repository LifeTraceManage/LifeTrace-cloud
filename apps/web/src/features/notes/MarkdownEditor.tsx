import { useEffect, useRef, type ReactNode } from "react";
import { autocompletion, type CompletionContext } from "@codemirror/autocomplete";
import { markdown } from "@codemirror/lang-markdown";
import { basicSetup, EditorView } from "codemirror";
import {
  Bold, Code2, Heading2, Italic, Link2, List, ListChecks, ListOrdered, Minus,
  Quote, Strikethrough, Table2,
} from "lucide-react";

type Draft = {
  value: string;
  dirty: boolean;
  updatedAt: string;
};

export interface WikiSuggestion {
  title: string;
  aliases?: string[];
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

const editorTheme = EditorView.theme({
  "&": {
    minHeight: "520px",
    backgroundColor: "hsl(var(--background))",
    color: "hsl(var(--foreground))",
    fontSize: "14px",
  },
  "&.cm-focused": {
    outline: "none",
  },
  ".cm-scroller": {
    minHeight: "520px",
    fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", \"Microsoft YaHei\", sans-serif",
    lineHeight: "1.75",
    overflow: "auto",
  },
  ".cm-content": {
    padding: "18px 22px 120px",
    caretColor: "hsl(var(--foreground))",
  },
  ".cm-line": {
    padding: "0",
  },
  ".cm-gutters": {
    display: "none",
  },
  ".cm-activeLine": {
    backgroundColor: "transparent",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
  },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "hsl(var(--accent)) !important",
  },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: "hsl(var(--foreground))",
  },
  ".cm-tooltip": {
    border: "1px solid hsl(var(--border))",
    borderRadius: "8px",
    backgroundColor: "hsl(var(--popover))",
    color: "hsl(var(--popover-foreground))",
    boxShadow: "0 10px 30px rgb(0 0 0 / 0.12)",
    overflow: "hidden",
  },
  ".cm-tooltip-autocomplete > ul > li": {
    padding: "6px 10px",
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "hsl(var(--accent))",
    color: "hsl(var(--accent-foreground))",
  },
});

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

export function MarkdownEditor({
  value,
  cacheKey,
  legacyCacheKey,
  cloudSaveRevision,
  wikiSuggestions = [],
  onChange,
  onSave,
  onSelectionChange,
}: MarkdownEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<EditorView | null>(null);
  const valueRef = useRef(value);
  const onChangeRef = useRef(onChange);
  const onSaveRef = useRef(onSave);
  const onSelectionChangeRef = useRef(onSelectionChange);
  const wikiSuggestionsRef = useRef(wikiSuggestions);
  const syncingRef = useRef(false);

  valueRef.current = value;
  onChangeRef.current = onChange;
  onSaveRef.current = onSave;
  onSelectionChangeRef.current = onSelectionChange;
  wikiSuggestionsRef.current = wikiSuggestions;

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const cached = readDraft(cacheKey, legacyCacheKey);
    const initialValue = cached?.dirty ? cached.value : valueRef.current;

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

    const editor = new EditorView({
      parent: host,
      doc: initialValue,
      extensions: [
        basicSetup,
        markdown(),
        EditorView.lineWrapping,
        editorTheme,
        autocompletion({
          activateOnTyping: true,
          override: [wikiCompletion],
        }),
        EditorView.contentAttributes.of({
          "aria-label": "Markdown 编辑器",
          spellcheck: "true",
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !syncingRef.current) {
            const next = update.state.doc.toString();
            valueRef.current = next;
            writeDraft(cacheKey, next, true);
            onChangeRef.current(next);
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
    });

    editorRef.current = editor;
    if (cached?.dirty && cached.value !== valueRef.current) {
      valueRef.current = cached.value;
      onChangeRef.current(cached.value);
    }

    return () => {
      onSelectionChangeRef.current?.("");
      editor.destroy();
      editorRef.current = null;
    };
  }, [cacheKey, legacyCacheKey]);

  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) return;
    const current = editor.state.doc.toString();
    if (current === value) return;

    syncingRef.current = true;
    editor.dispatch({
      changes: { from: 0, to: current.length, insert: value },
    });
    syncingRef.current = false;
  }, [value]);

  useEffect(() => {
    if (!cloudSaveRevision) return;
    writeDraft(cacheKey, valueRef.current, false);
    clearLegacyDraft(legacyCacheKey);
  }, [cacheKey, cloudSaveRevision, legacyCacheKey]);

  function wrapSelection(prefix: string, suffix = prefix, placeholder = "文本") {
    const editor = editorRef.current;
    if (!editor) return;

    const selection = editor.state.selection.main;
    const selected = selection.empty
      ? placeholder
      : editor.state.doc.sliceString(selection.from, selection.to);
    const insert = prefix + selected + suffix;
    const selectedFrom = selection.from + prefix.length;

    editor.dispatch({
      changes: { from: selection.from, to: selection.to, insert },
      selection: {
        anchor: selectedFrom,
        head: selectedFrom + selected.length,
      },
    });
    editor.focus();
  }

  function prefixLines(prefix: string) {
    const editor = editorRef.current;
    if (!editor) return;

    const selection = editor.state.selection.main;
    const start = editor.state.doc.lineAt(selection.from).from;
    const end = editor.state.doc.lineAt(selection.to).to;
    const source = editor.state.doc.sliceString(start, end);
    const insert = source.split("\n").map((line) => prefix + line).join("\n");

    editor.dispatch({
      changes: { from: start, to: end, insert },
      selection: { anchor: start + prefix.length, head: start + insert.length },
    });
    editor.focus();
  }

  function insertSnippet(snippet: string) {
    const editor = editorRef.current;
    if (!editor) return;

    const selection = editor.state.selection.main;
    editor.dispatch({
      changes: { from: selection.from, to: selection.to, insert: snippet },
      selection: { anchor: selection.from + snippet.length },
    });
    editor.focus();
  }

  return <div className="min-w-0 overflow-hidden rounded-md border bg-background" data-testid="markdown-editor">
    <div className="scrollbar-thin flex items-center gap-0.5 overflow-x-auto border-b bg-muted/20 px-2 py-1">
      <ToolButton label="二级标题" onClick={() => prefixLines("## ")}><Heading2 size={15} /></ToolButton>
      <ToolButton label="粗体" onClick={() => wrapSelection("**")}><Bold size={15} /></ToolButton>
      <ToolButton label="斜体" onClick={() => wrapSelection("_")}><Italic size={15} /></ToolButton>
      <ToolButton label="删除线" onClick={() => wrapSelection("~~")}><Strikethrough size={15} /></ToolButton>
      <ToolButton label="链接" onClick={() => wrapSelection("[", "](https://)", "链接文字")}><Link2 size={15} /></ToolButton>
      <span className="mx-1 h-4 w-px shrink-0 bg-border" />
      <ToolButton label="无序列表" onClick={() => prefixLines("- ")}><List size={15} /></ToolButton>
      <ToolButton label="有序列表" onClick={() => prefixLines("1. ")}><ListOrdered size={15} /></ToolButton>
      <ToolButton label="任务列表" onClick={() => prefixLines("- [ ] ")}><ListChecks size={15} /></ToolButton>
      <ToolButton label="引用" onClick={() => prefixLines("> ")}><Quote size={15} /></ToolButton>
      <span className="mx-1 h-4 w-px shrink-0 bg-border" />
      <ToolButton label="行内代码" onClick={() => wrapSelection("\`")}><Code2 size={15} /></ToolButton>
      <ToolButton label="分隔线" onClick={() => insertSnippet("\n---\n")}><Minus size={15} /></ToolButton>
      <ToolButton
        label="表格"
        onClick={() => insertSnippet("\n| 列 1 | 列 2 |\n| --- | --- |\n|  |  |\n")}
      ><Table2 size={15} /></ToolButton>
      <div className="ml-auto hidden shrink-0 px-2 text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground sm:block">Markdown</div>
    </div>
    <div ref={hostRef} />
  </div>;
}
