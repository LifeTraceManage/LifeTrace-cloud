// @vitest-environment jsdom

import { fireEvent, render, waitFor } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { MarkdownEditor } from "./MarkdownEditor";

function TableHarness() {
  const [value, setValue] = useState([
    "| Name | Value |",
    "| --- | --- |",
    "| A | B |",
  ].join("\n"));

  return <MarkdownEditor
    value={value}
    cacheKey="test:markdown:table"
    cloudSaveRevision={0}
    onChange={setValue}
  />;
}

describe("MarkdownEditor table quick insert", () => {
  it("keeps the side plus button clickable and inserts a row", async () => {
    const { container } = render(<TableHarness />);

    await waitFor(() => {
      expect(container.querySelector(".cm-table-widget tbody td")).not.toBeNull();
    });

    const firstCell = container.querySelector(".cm-table-widget tbody td");
    expect(firstCell).not.toBeNull();
    fireEvent.mouseMove(firstCell!);

    const rowInserter = await waitFor(() => {
      const button = container.querySelector<HTMLButtonElement>(".cm-table-inserter--row");
      expect(button).not.toBeNull();
      expect(button?.style.display).toBe("flex");
      return button!;
    });

    const tableWrap = container.querySelector<HTMLElement>("[data-tablev2-from]");
    expect(tableWrap).not.toBeNull();

    // Reproduce the real pointer path: the quick-insert button sits outside
    // the table, so the mouse must cross the wrapper gutter before reaching it.
    fireEvent.mouseMove(tableWrap!);
    expect(rowInserter.style.display).toBe("flex");

    fireEvent.mouseMove(rowInserter);
    expect(rowInserter.style.display).toBe("flex");

    fireEvent.mouseDown(rowInserter);
    fireEvent.click(rowInserter);

    await waitFor(() => {
      expect(container.querySelectorAll(".cm-table-widget tbody tr")).toHaveLength(2);
    });
  });

  it("uses the flat LifeTrace Markdown surface and theme bridge", async () => {
    const { container } = render(<TableHarness />);
    await waitFor(() => expect(container.querySelector("[data-testid=markdown-editor]")).not.toBeNull());
    const root = container.querySelector<HTMLElement>("[data-testid=markdown-editor]");
    expect(root?.classList.contains("lifetrace-markdown-theme")).toBe(true);
    expect(root?.dataset.editorSurface).toBe("flat");
    expect(root?.classList.contains("rounded-md")).toBe(false);
    expect(root?.classList.contains("border")).toBe(false);
  });
});
