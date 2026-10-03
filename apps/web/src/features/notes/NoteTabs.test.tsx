// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { NoteTabs } from "./NoteTabs";
import type { JsonEntity } from "../../services/core";

function note(id: string, title: string): JsonEntity {
  return {
    meta: {
      id,
      userId: "user-1",
      createdAt: "2026-10-03T00:00:00.000Z",
      updatedAt: "2026-10-03T00:00:00.000Z",
      localVersion: 1,
    },
    title,
  };
}

describe("NoteTabs", () => {
  it("renders flat tabs with an underline active state instead of card pills", () => {
    render(
      <NoteTabs
        notes={[note("a", "Alpha"), note("b", "Beta")]}
        openedIds={["a", "b"]}
        activeId="a"
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const tabs = screen.getByTestId("note-tabs");
    const active = screen.getByRole("button", { name: "Alpha" }).parentElement;
    const inactive = screen.getByRole("button", { name: "Beta" }).parentElement;

    expect(tabs.classList.contains("border-b")).toBe(true);
    expect(active?.classList.contains("border-primary")).toBe(true);
    expect(active?.classList.contains("rounded-md")).toBe(false);
    expect(active?.classList.contains("bg-card")).toBe(false);
    expect(inactive?.classList.contains("border-transparent")).toBe(true);
  });
});
