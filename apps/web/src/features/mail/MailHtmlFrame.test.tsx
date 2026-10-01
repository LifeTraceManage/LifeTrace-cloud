// @vitest-environment jsdom

import { describe, expect, it } from "vitest";

import { measureMailDocumentHeight } from "./MailHtmlFrame";

describe("measureMailDocumentHeight", () => {
  it("ignores the transient iframe state before documentElement exists", () => {
    const transient = {
      documentElement: null,
      body: null,
    } as unknown as Document;

    expect(measureMailDocumentHeight(transient)).toBeNull();
    expect(measureMailDocumentHeight(null)).toBeNull();
  });

  it("returns at least the minimum render height", () => {
    const document = new DOMParser().parseFromString("<html><body>mail</body></html>", "text/html");
    Object.defineProperty(document.documentElement, "scrollHeight", { value: 80, configurable: true });
    Object.defineProperty(document.body, "scrollHeight", { value: 90, configurable: true });

    expect(measureMailDocumentHeight(document)).toBe(120);
  });
});
