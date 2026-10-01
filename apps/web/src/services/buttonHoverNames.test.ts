// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

import { installButtonHoverNames } from "./buttonHoverNames";

afterEach(() => {
  document.body.innerHTML = "";
});

describe("installButtonHoverNames", () => {
  it("adds hover names to native icon-only buttons", () => {
    const host = document.createElement("div");
    host.innerHTML = '<button aria-label="刷新"><svg></svg></button>';
    document.body.append(host);

    const dispose = installButtonHoverNames(host);
    const button = host.querySelector("button");

    expect(button?.getAttribute("title")).toBe("刷新");
    dispose();
  });

  it("handles icon buttons added after startup", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const dispose = installButtonHoverNames(host);

    const button = document.createElement("button");
    button.setAttribute("aria-label", "关闭");
    button.innerHTML = "<svg></svg>";
    host.append(button);

    await new Promise<void>((resolve) => queueMicrotask(resolve));
    expect(button.getAttribute("title")).toBe("关闭");
    dispose();
  });

  it("does not duplicate hover names on buttons with visible text", () => {
    const host = document.createElement("div");
    host.innerHTML = '<button aria-label="保存">保存</button>';
    document.body.append(host);

    const dispose = installButtonHoverNames(host);
    const button = host.querySelector("button");

    expect(button?.hasAttribute("title")).toBe(false);
    dispose();
  });
});
