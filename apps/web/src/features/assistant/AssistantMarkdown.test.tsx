// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AssistantMarkdown } from "./AssistantMarkdown";

describe("AssistantMarkdown", () => {
  it("renders common Markdown and GFM content", () => {
    render(
      <AssistantMarkdown content={"**重点**\n\n- 一\n- 二\n\n| A | B |\n| --- | --- |\n| 1 | 2 |"} />,
    );

    expect(screen.getByText("重点").tagName).toBe("STRONG");
    expect(screen.getByText("一").closest("li")).not.toBeNull();
    expect(screen.getByRole("table")).not.toBeNull();
  });

  it("does not interpret raw HTML from model output", () => {
    const { container } = render(
      <AssistantMarkdown content={'<script>alert("x")</script>\n\n<b>not raw html</b>'} />,
    );

    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("b")).toBeNull();
  });

  it("opens links without giving the new page opener access", () => {
    render(<AssistantMarkdown content={"[OpenAI](https://openai.com)"} />);
    const link = screen.getByRole("link", { name: "OpenAI" });
    expect(link.getAttribute("target")).toBe("_blank");
    expect(link.getAttribute("rel")).toContain("noopener");
  });
});
