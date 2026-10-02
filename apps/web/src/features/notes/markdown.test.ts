import { describe, expect, it } from "vitest";
import { appendMarkdownLink, extractMarkdownHeadings, extractWikiLinks, markdownSummary, plainTextFromMarkdown, uniqueWikiTargets } from "./markdown";

describe("markdown helpers", () => {
  it("creates readable plain text and summaries", () => {
    const source = "# Tube MPC\n\n- **Robust** tracking\n- [[SMF|Set Membership Filter]]";
    expect(plainTextFromMarkdown(source)).toContain("Tube MPC");
    expect(plainTextFromMarkdown(source)).toContain("Set Membership Filter");
    expect(markdownSummary(source, 8)).toBe("Tube MPC");
  });

  it("extracts wiki links and ignores code", () => {
    const source = [
      "Link [[Tube MPC]] and [[SMF|Set Membership Filter]].",
      "",
      "`[[Inline Code]]`",
      "",
      "```text",
      "[[Code Block]]",
      "```",
    ].join("\n");
    expect(extractWikiLinks(source)).toEqual([
      { target: "Tube MPC", label: "Tube MPC", raw: "[[Tube MPC]]" },
      { target: "SMF", label: "Set Membership Filter", raw: "[[SMF|Set Membership Filter]]" },
    ]);
  });

  it("deduplicates wiki targets case-insensitively", () => {
    expect(uniqueWikiTargets("[[MPC]] [[mpc]] [[SMF]]")).toEqual(["MPC", "SMF"]);
  });

  it("builds a live outline and ignores fenced examples", () => {
    const source = "# Title\n\n## Section **one**\n\n~~~md\n# Not a heading\n~~~\n\n### Detail";
    expect(extractMarkdownHeadings(source)).toEqual([
      { level: 1, text: "Title", line: 1 },
      { level: 2, text: "Section one", line: 3 },
      { level: 3, text: "Detail", line: 9 },
    ]);
  });

  it("appends generated links without corrupting markdown", () => {
    expect(appendMarkdownLink("# Note\n", "[report.pdf](attachment://abc)")).toBe("# Note\n\n[report.pdf](attachment://abc)\n");
  });
});
