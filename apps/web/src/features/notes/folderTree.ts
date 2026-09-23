import { number, text } from "../../lib/entities";
import type { JsonEntity } from "../../services/core";

export interface FolderRow {
  folder: JsonEntity;
  depth: number;
  path: string;
}

function ordered(values: JsonEntity[]): JsonEntity[] {
  return [...values].sort((a, b) =>
    number(a, "sortOrder") - number(b, "sortOrder")
    || text(a, "name").localeCompare(text(b, "name"), "zh-CN")
  );
}

/**
 * Flatten the parentFolderId graph for UI rendering. Orphans and accidental
 * cycles are appended as roots so malformed synced data cannot disappear or
 * recurse forever.
 */
export function flattenNoteFolders(folders: JsonEntity[]): FolderRow[] {
  const byId = new Map(folders.map((folder) => [folder.meta.id, folder]));
  const children = new Map<string | null, JsonEntity[]>();
  for (const folder of folders) {
    const rawParent = text(folder, "parentFolderId");
    const parent = rawParent && rawParent !== folder.meta.id && byId.has(rawParent)
      ? rawParent
      : null;
    const values = children.get(parent) ?? [];
    values.push(folder);
    children.set(parent, values);
  }

  const result: FolderRow[] = [];
  const visited = new Set<string>();

  function visit(folder: JsonEntity, depth: number, parents: string[]) {
    if (visited.has(folder.meta.id)) return;
    visited.add(folder.meta.id);
    const name = text(folder, "name", "未命名");
    const path = [...parents, name];
    result.push({ folder, depth, path: path.join(" / ") });
    for (const child of ordered(children.get(folder.meta.id) ?? [])) {
      visit(child, depth + 1, path);
    }
  }

  for (const root of ordered(children.get(null) ?? [])) visit(root, 0, []);

  // A pure cycle may have no root. Preserve accessibility by rendering any
  // unvisited node at root level; visited prevents recursion loops.
  for (const folder of ordered(folders)) {
    if (!visited.has(folder.meta.id)) visit(folder, 0, []);
  }
  return result;
}

export function folderDescendantIds(folders: JsonEntity[], folderId: string): Set<string> {
  const children = new Map<string, string[]>();
  for (const folder of folders) {
    const parent = text(folder, "parentFolderId");
    if (!parent) continue;
    const values = children.get(parent) ?? [];
    values.push(folder.meta.id);
    children.set(parent, values);
  }
  const result = new Set<string>();
  const queue = [...(children.get(folderId) ?? [])];
  while (queue.length) {
    const id = queue.shift()!;
    if (result.has(id) || id === folderId) continue;
    result.add(id);
    queue.push(...(children.get(id) ?? []));
  }
  return result;
}
