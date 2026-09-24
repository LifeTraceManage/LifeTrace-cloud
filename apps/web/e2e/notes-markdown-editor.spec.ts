import { expect, test, type Page, type Route } from "@playwright/test";

const now = "2026-08-19T02:00:00.000Z";
const legacyCacheKey = "lifetrace:vditor:user-1:note-1";
const markdownCacheKey = "lifetrace:notes:draft:user-1:note-1:markdown";
type PushBody = { changes?: Array<{ entityType?: string; payload?: Record<string, unknown> }> };

function meta(id: string) {
  return { id, userId: "user-1", createdAt: now, updatedAt: now, localVersion: 1, serverVersion: "1", modifiedByDevice: "web-test" };
}

async function json(route: Route, payload: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(payload) });
}

async function installMocks(page: Page, pushes: PushBody[], cloudMarkdown = "# Cloud note\n\nCloud body") {
  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/v1/web/session") return json(route, { user: { id: "user-1", email: "tester@example.com", displayName: "Web Tester" }, session: { id: "session-1", appId: "lifetrace-web", deviceId: "web-test", scopes: ["sync:read", "sync:write"], idleExpiresAt: "2026-08-20T00:00:00.000Z", absoluteExpiresAt: "2026-08-26T00:00:00.000Z", publicDevice: false }, csrfToken: "csrf-test" });
    if (path === "/api/v1/sync/snapshot") return json(route, { snapshotId: "snapshot-1", snapshotCursor: "cursor-1", items: [{ entityType: "note.note", entityId: "note-1", serverVersion: "1", payload: { meta: meta("note-1"), noteType: "quick", title: "Markdown note", contentMarkdown: cloudMarkdown, contentText: "Cloud note Cloud body", contentHtml: "", contentJson: { type: "markdown", source: cloudMarkdown, editor: "codemirror" }, summary: "Cloud note Cloud body", isPinned: false, isFavorite: false, isArchived: false, folderId: null } }], nextPageToken: null, completed: true });
    if (path === "/api/v1/sync/pull") return json(route, { changes: [], nextCursor: "cursor-2", hasMore: false });
    if (path === "/api/v1/sync/push") {
      const body = route.request().postDataJSON() as PushBody; pushes.push(body);
      return json(route, { results: (body.changes ?? []).map((change, index) => ({ changeId: "change-" + index, entityType: change.entityType, entityId: "note-1", status: "accepted", serverVersion: "server-" + (index + 2) })) });
    }
    return json(route, {});
  });
}

function noteMarkdownPush(pushes: PushBody[]) {
  for (const body of pushes) {
    for (const change of body.changes ?? []) {
      if (change.entityType === "note.note" && typeof change.payload?.contentMarkdown === "string") {
        return change.payload.contentMarkdown;
      }
    }
  }
  return null;
}

test("CodeMirror edits Markdown and autosaves the note to LifeTrace Cloud", async ({ page }) => {
  const pushes: PushBody[] = [];
  await installMocks(page, pushes);
  await page.goto("/app/notes");

  const editor = page.getByTestId("markdown-editor").locator(".cm-content");
  await expect(editor).toBeVisible();
  await expect(editor).toContainText("Cloud body");
  await expect(page.getByRole("button", { name: "粗体" })).toBeVisible();

  await editor.fill("# Local CodeMirror edit\n\n- [x] cloud autosave");
  await expect.poll(() => noteMarkdownPush(pushes), { timeout: 6_000 }).toContain("Local CodeMirror edit");
  expect(noteMarkdownPush(pushes)).toContain("cloud autosave");
});

test("dirty CodeMirror localStorage draft is restored and promoted to Cloud autosave", async ({ page }) => {
  const pushes: PushBody[] = [];
  await page.addInitScript(({ key }) => {
    localStorage.setItem(key, JSON.stringify({
      value: "# Recovered draft\n\nLocal unsaved text",
      dirty: true,
      updatedAt: new Date().toISOString(),
    }));
  }, { key: markdownCacheKey });

  await installMocks(page, pushes, "# Cloud version\n\nOlder cloud text");
  await page.goto("/app/notes");

  const editor = page.getByTestId("markdown-editor").locator(".cm-content");
  await expect(editor).toContainText("Recovered draft");
  await expect.poll(() => noteMarkdownPush(pushes), { timeout: 6_000 }).toContain("Recovered draft");
  expect(noteMarkdownPush(pushes)).toContain("Local unsaved text");
});

test("dirty legacy Vditor draft migrates into CodeMirror and Cloud autosave", async ({ page }) => {
  const pushes: PushBody[] = [];
  await page.addInitScript(({ key }) => {
    localStorage.setItem(key, "# Legacy recovered draft\n\nUnsaved before editor migration");
    localStorage.setItem(key + ":meta", JSON.stringify({ dirty: true, updatedAt: new Date().toISOString() }));
  }, { key: legacyCacheKey });

  await installMocks(page, pushes, "# Cloud version\n\nOlder cloud text");
  await page.goto("/app/notes");

  const editor = page.getByTestId("markdown-editor").locator(".cm-content");
  await expect(editor).toContainText("Legacy recovered draft");
  await expect.poll(() => noteMarkdownPush(pushes), { timeout: 6_000 }).toContain("Legacy recovered draft");
  expect(noteMarkdownPush(pushes)).toContain("Unsaved before editor migration");
});
