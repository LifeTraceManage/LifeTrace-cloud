import { describe, expect, it } from "vitest";
import { normalizeApiBase, REQUESTED_SCOPES } from "../core";

describe("reused cloud contract helpers", () => {
  it("normalizes API base URLs", () => expect(normalizeApiBase("https://example.test///")).toBe("https://example.test"));
  it("requests entity-link scopes for web sync", () => {
    expect(REQUESTED_SCOPES).toContain("links:read");
    expect(REQUESTED_SCOPES).toContain("links:write");
  });
});
