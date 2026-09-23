import { describe, expect, it } from "vitest";
import type { JsonEntity } from "../../services/core";
import { flattenNoteFolders, folderDescendantIds } from "./folderTree";

function folder(id: string, name: string, parentFolderId: string | null = null, sortOrder = 0): JsonEntity {
  const now = "2026-09-23T00:00:00.000Z";
  return {
    meta: { id, userId: "u", createdAt: now, updatedAt: now, localVersion: 1 },
    name,
    parentFolderId,
    sortOrder,
  };
}

describe("note folder tree", () => {
  it("flattens nested folders with stable depth and path", () => {
    const rows = flattenNoteFolders([
      folder("b", "SMF", "a"),
      folder("a", "Research"),
      folder("c", "Papers", "b"),
    ]);
    expect(rows.map((row) => [row.folder.meta.id, row.depth, row.path])).toEqual([
      ["a", 0, "Research"],
      ["b", 1, "Research / SMF"],
      ["c", 2, "Research / SMF / Papers"],
    ]);
  });

  it("does not lose orphan or cyclic folders", () => {
    const rows = flattenNoteFolders([
      folder("orphan", "Orphan", "missing"),
      folder("a", "A", "b"),
      folder("b", "B", "a"),
    ]);
    expect(new Set(rows.map((row) => row.folder.meta.id))).toEqual(new Set(["orphan", "a", "b"]));
  });

  it("collects descendants without looping", () => {
    expect(folderDescendantIds([
      folder("a", "A"),
      folder("b", "B", "a"),
      folder("c", "C", "b"),
    ], "a")).toEqual(new Set(["b", "c"]));
  });
});
