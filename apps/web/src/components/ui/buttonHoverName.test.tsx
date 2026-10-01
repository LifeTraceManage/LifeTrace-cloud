import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Button } from "./index";

describe("Button hover names", () => {
  it("uses aria-label as the hover title for icon buttons", () => {
    const html = renderToStaticMarkup(
      <Button size="icon" aria-label="删除">
        <svg aria-hidden="true" />
      </Button>,
    );

    expect(html).toContain('aria-label="删除"');
    expect(html).toContain('title="删除"');
  });

  it("keeps an explicit title when one is provided", () => {
    const html = renderToStaticMarkup(
      <Button size="icon" aria-label="删除" title="永久删除">
        <svg aria-hidden="true" />
      </Button>,
    );

    expect(html).toContain('title="永久删除"');
  });
});
