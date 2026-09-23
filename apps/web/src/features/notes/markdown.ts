export interface WikiLink {
  target: string;
  label: string;
  raw: string;
}

export function plainTextFromMarkdown(markdown: string): string {
  return markdown
    .replace(/```[\s\S]*?```/g, (block) => block.replace(/^```[^\n]*\n?|```$/g, " "))
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g, (_match, target: string, label?: string) => (label || target).trim())
    .replace(/^\s{0,3}#{1,6}\s+/gm, "")
    .replace(/^\s*>\s?/gm, "")
    .replace(/^\s*[-+*]\s+(?:\[[ xX]\]\s*)?/gm, "")
    .replace(/^\s*\d+[.)]\s+/gm, "")
    .replace(/\*\*|__|~~|`|\*/g, "")
    .replace(/<[^>]+>/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function markdownSummary(markdown: string, limit = 160): string {
  return plainTextFromMarkdown(markdown).slice(0, limit);
}

/**
 * Parse Obsidian/SilverBullet-style wiki links while ignoring fenced and inline
 * code. Supports [[Target]] and [[Target|Alias]]. Resolution is intentionally
 * kept separate so the parser remains deterministic and easy to test.
 */
export function extractWikiLinks(markdown: string): WikiLink[] {
  const results: WikiLink[] = [];
  const token = /```[\s\S]*?```|`[^`\n]*`|\[\[([^\[\]\n|]+)(?:\|([^\[\]\n]+))?\]\]/g;
  let match: RegExpExecArray | null;
  while ((match = token.exec(markdown)) !== null) {
    if (!match[1]) continue;
    const target = match[1].trim();
    if (!target) continue;
    results.push({
      target,
      label: (match[2] || target).trim(),
      raw: match[0],
    });
  }
  return results;
}

export function uniqueWikiTargets(markdown: string): string[] {
  const seen = new Set<string>();
  const values: string[] = [];
  for (const link of extractWikiLinks(markdown)) {
    const key = link.target.toLocaleLowerCase("zh-CN");
    if (seen.has(key)) continue;
    seen.add(key);
    values.push(link.target);
  }
  return values;
}

export function appendMarkdownLink(markdown: string, value: string): string {
  const trimmed = markdown.replace(/\s+$/, "");
  return `${trimmed}${trimmed ? "\n\n" : ""}${value}\n`;
}
