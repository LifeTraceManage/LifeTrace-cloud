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


test("notes workspace uses one left sidebar and a right outline/properties inspector", async ({ page }) => {
  const pushes: PushBody[] = [];
  await installMocks(page, pushes, "# Product note\n\n## Goals\n\nBody");
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/app/notes");

  const sidebar = page.getByTestId("notes-sidebar");
  const inspector = page.getByTestId("notes-inspector");
  await expect(sidebar).toBeVisible();
  await expect(sidebar.getByRole("navigation", { name: "笔记导航" })).toBeVisible();
  await expect(sidebar).toContainText("Markdown note");
  await expect(inspector).toBeVisible();
  await expect(inspector.getByTestId("note-outline")).toContainText("Product note");
  await expect(inspector.getByTestId("note-outline")).toContainText("Goals");
  await expect(inspector.getByTestId("note-properties")).toBeVisible();

  const editorColumn = page.getByTestId("notes-editor-column");
  await expect.poll(() => sidebar.evaluate((node) => getComputedStyle(node).overflowY)).toBe("auto");
  await expect.poll(() => editorColumn.evaluate((node) => getComputedStyle(node).overflowY)).toBe("auto");
  await expect.poll(() => inspector.evaluate((node) => getComputedStyle(node).overflowY)).toBe("auto");
  await expect(inspector.locator("[data-inspector-section]")).toHaveCount(4);
});

test("all common Markdown constructs render inline while the outline updates immediately", async ({ page }) => {
  const pushes: PushBody[] = [];
  await installMocks(page, pushes);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/app/notes");

  const editorRoot = page.getByTestId("markdown-editor");
  const editor = editorRoot.locator(".cm-content");
  await expect(editorRoot).toHaveAttribute("data-live-preview", "true");
  await expect(editorRoot).toHaveAttribute("data-render-engine", "full");

  await editor.fill([
    "# Live heading",
    "",
    "## Instant outline",
    "",
    "**Bold** *italic* ~~strike~~ `inline` [docs](https://example.com)",
    "",
    "- [x] completed task",
    "1. ordered item",
    "> quoted text",
    "",
    "| Name | Value |",
    "| --- | --- |",
    "| A | B |",
    "",
    "![sample](https://example.com/image.png)",
    "",
    "---",
  ].join("\n"));

  const inspector = page.getByTestId("notes-inspector");
  await expect(inspector.getByTestId("note-outline")).toContainText("Live heading");
  await expect(inspector.getByTestId("note-outline")).toContainText("Instant outline");

  await expect(editorRoot.locator(".cm-task-checkbox")).toBeVisible();
  await expect(editorRoot.locator(".cm-ordered-marker")).toContainText("1.");
  await expect(editorRoot.locator(".cm-blockquote")).toContainText("quoted text");
  const table = editorRoot.locator(".cm-table-widget");
  await expect(table).toContainText("Name");
  await expect(table).toContainText("Value");

  await table.locator("tbody td").first().hover();
  const rowInserter = editorRoot.locator(".cm-table-inserter--row");
  await expect(rowInserter).toBeVisible();
  await rowInserter.click();
  await expect(table.locator("tbody tr")).toHaveCount(2);

  await expect(editorRoot.locator(".cm-image-widget")).toHaveAttribute("alt", "sample");
  await expect(editorRoot.locator(".cm-hr-widget")).toBeVisible();

  await expect(editor).toContainText("Bold italic strike inline docs");
  await expect(editor).not.toContainText("**Bold**");
  await expect(editor).not.toContainText("[docs](https://example.com)");
});

test("Markdown editor follows LifeTrace dark theme tokens", async ({ page }) => {
  const pushes: PushBody[] = [];
  await installMocks(page, pushes, [
    "# Dark note",
    "",
    "> quote",
    "",
    "| Name | Value |",
    "| --- | --- |",
    "| A | B |",
  ].join("\n"));
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/app/notes");

  const html = page.locator("html");
  if (await html.getAttribute("data-theme") !== "dark") {
    await page.getByRole("button", { name: "切换主题" }).click();
  }
  await expect(html).toHaveAttribute("data-theme", "dark");

  const editorRoot = page.getByTestId("markdown-editor");
  const themeState = await editorRoot.evaluate((node) => {
    const content = node.querySelector(".cm-content");
    const tableHead = node.querySelector(".cm-table-widget thead th");
    const bodyStyle = getComputedStyle(document.body);
    const rootStyle = getComputedStyle(node);
    return {
      bodyColor: bodyStyle.color,
      contentColor: content ? getComputedStyle(content).color : "",
      editorBackground: getComputedStyle(node).backgroundColor,
      tableHeadBackground: tableHead ? getComputedStyle(tableHead).backgroundColor : "",
      cdsTextPrimary: rootStyle.getPropertyValue("--cds-text-primary").trim(),
      cdsLayerAccent: rootStyle.getPropertyValue("--cds-layer-accent-01").trim(),
    };
  });

  expect(themeState.contentColor).toBe(themeState.bodyColor);
  expect(themeState.cdsTextPrimary).toContain("var(--foreground)");
  expect(themeState.cdsLayerAccent).toContain("var(--muted)");
  expect(themeState.editorBackground).not.toBe("rgb(255, 255, 255)");
  expect(themeState.tableHeadBackground).not.toBe("rgb(232, 232, 232)");
});

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
