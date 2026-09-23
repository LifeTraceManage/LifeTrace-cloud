import { describe, expect, it } from "vitest";
import { mergeNoteProperties, parseFrontmatter, storedNoteProperties } from "./properties";

describe("note properties", () => {
  it("parses supported YAML-like frontmatter and removes it from the editor body", () => {
    const parsed = parseFrontmatter([
      "---",
      "title: Tube MPC",
      "status: research",
      "source: paper",
      "favorite: true",
      "tags:",
      "  - MPC",
      "  - SMF",
      "aliases: [Robust MPC, Tube control]",
      "---",
      "",
      "# Body",
    ].join("\n"));

    expect(parsed.found).toBe(true);
    expect(parsed.body).toBe("# Body");
    expect(parsed.properties).toEqual({
      title: "Tube MPC",
      status: "research",
      source: "paper",
      favorite: true,
      tags: ["MPC", "SMF"],
      aliases: ["Robust MPC", "Tube control"],
    });
  });

  it("returns plain markdown unchanged when frontmatter is absent", () => {
    expect(parseFrontmatter("# Note")).toEqual({
      body: "# Note",
      properties: { tags: [] },
      found: false,
    });
  });

  it("reads and merges structured contentJson properties", () => {
    const stored = storedNoteProperties({
      type: "markdown",
      properties: { status: "draft", source: "web", aliases: ["Old"] },
    });
    expect(mergeNoteProperties(stored, {
      status: "research",
      aliases: ["New"],
      tags: [],
    })).toEqual({
      status: "research",
      source: "web",
      aliases: ["New"],
    });
  });
});
