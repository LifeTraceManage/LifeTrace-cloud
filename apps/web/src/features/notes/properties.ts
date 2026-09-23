export interface NoteProperties {
  status: string;
  source: string;
  aliases: string[];
}

export interface FrontmatterProperties extends Partial<NoteProperties> {
  title?: string;
  tags: string[];
  favorite?: boolean;
}

export interface ParsedFrontmatter {
  body: string;
  properties: FrontmatterProperties;
  found: boolean;
}

function unquote(value: string): string {
  const trimmed = value.trim();
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"'))
    || (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}

function uniqueStrings(values: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const raw of values) {
    const value = unquote(raw).trim();
    if (!value) continue;
    const key = value.toLocaleLowerCase("zh-CN");
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(value);
  }
  return result;
}

function inlineList(value: string): string[] {
  const trimmed = value.trim();
  if (!trimmed.startsWith("[") || !trimmed.endsWith("]")) {
    return trimmed ? [trimmed] : [];
  }
  return uniqueStrings(trimmed.slice(1, -1).split(","));
}

export function parseFrontmatter(markdown: string): ParsedFrontmatter {
  const normalized = markdown.replace(/^\uFEFF/, "");
  if (!normalized.startsWith("---\n") && !normalized.startsWith("---\r\n")) {
    return {
      body: markdown,
      properties: { tags: [] },
      found: false,
    };
  }

  const lines = normalized.split(/\r?\n/);
  let end = -1;
  for (let index = 1; index < lines.length; index += 1) {
    if (lines[index].trim() === "---") {
      end = index;
      break;
    }
  }
  if (end < 0) {
    return {
      body: markdown,
      properties: { tags: [] },
      found: false,
    };
  }

  const properties: FrontmatterProperties = { tags: [] };
  let listKey: "tags" | "aliases" | null = null;
  const lists: Record<"tags" | "aliases", string[]> = { tags: [], aliases: [] };

  for (const rawLine of lines.slice(1, end)) {
    const line = rawLine.trimEnd();
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    if (listKey && /^\s*-\s+/.test(line)) {
      lists[listKey].push(trimmed.replace(/^[-]\s*/, ""));
      continue;
    }

    const separator = trimmed.indexOf(":");
    if (separator < 0) {
      listKey = null;
      continue;
    }
    const key = trimmed.slice(0, separator).trim().toLocaleLowerCase();
    const value = trimmed.slice(separator + 1).trim();
    listKey = null;

    if (key === "tags" || key === "aliases") {
      const target = key as "tags" | "aliases";
      if (value) lists[target].push(...inlineList(value));
      else listKey = target;
      continue;
    }

    if (key === "title" && value) properties.title = unquote(value);
    if (key === "status" && value) properties.status = unquote(value);
    if (key === "source" && value) properties.source = unquote(value);
    if (key === "favorite" && value) {
      const lower = value.toLocaleLowerCase();
      if (["true", "yes", "1"].includes(lower)) properties.favorite = true;
      if (["false", "no", "0"].includes(lower)) properties.favorite = false;
    }
  }

  properties.tags = uniqueStrings(lists.tags);
  if (lists.aliases.length) properties.aliases = uniqueStrings(lists.aliases);

  const body = lines.slice(end + 1).join("\n").replace(/^\n+/, "");
  return { body, properties, found: true };
}

export function storedNoteProperties(contentJson: unknown): NoteProperties {
  if (!contentJson || typeof contentJson !== "object" || Array.isArray(contentJson)) {
    return { status: "", source: "", aliases: [] };
  }
  const properties = (contentJson as Record<string, unknown>).properties;
  if (!properties || typeof properties !== "object" || Array.isArray(properties)) {
    return { status: "", source: "", aliases: [] };
  }
  const record = properties as Record<string, unknown>;
  return {
    status: typeof record.status === "string" ? record.status : "",
    source: typeof record.source === "string" ? record.source : "",
    aliases: Array.isArray(record.aliases)
      ? uniqueStrings(record.aliases.filter((value): value is string => typeof value === "string"))
      : [],
  };
}

export function mergeNoteProperties(
  stored: NoteProperties,
  imported: FrontmatterProperties,
): NoteProperties {
  return {
    status: imported.status ?? stored.status,
    source: imported.source ?? stored.source,
    aliases: imported.aliases?.length ? uniqueStrings(imported.aliases) : stored.aliases,
  };
}

export function noteAliases(contentJson: unknown): string[] {
  return storedNoteProperties(contentJson).aliases;
}
