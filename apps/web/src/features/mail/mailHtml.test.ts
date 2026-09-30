// @vitest-environment jsdom

import { describe, expect, it } from "vitest";

import { renderableMailHtml } from "./mailHtml";
import type { MailMessageDetail } from "./types";

function message(html: string): MailMessageDetail {
  return {
    id: "m1",
    accountId: "a1",
    subject: "test",
    from: [],
    to: [],
    sentAt: new Date(0).toISOString(),
    isRead: true,
    isStarred: false,
    categoryIds: [],
    html,
    text: null,
    attachments: [
      {
        id: "att1",
        messageId: "m1",
        filename: "logo.png",
        contentType: "image/png",
        contentId: "<logo@example>",
        downloadUrl: "/api/v1/mail/attachments/att1/content",
      },
    ],
  };
}

describe("renderableMailHtml", () => {
  it("keeps remote images and rewrites CID images", () => {
    const html = renderableMailHtml(message(
      '<p>Hello</p><img src="https://images.example/banner.png"><img src="cid:logo@example">'
    ));

    expect(html).toContain('src="https://images.example/banner.png"');
    expect(html).toContain('src="/api/v1/mail/attachments/att1/content"');
    expect(html).toContain('loading="lazy"');
    expect(html).toContain('referrerpolicy="no-referrer"');
  });

  it("drops unresolved CID sources instead of issuing a bogus request", () => {
    const html = renderableMailHtml(message('<img src="cid:missing@example">'));
    expect(html).not.toContain("cid:missing@example");
  });

  it("keeps email layout styles inside the isolated document", () => {
    const html = renderableMailHtml(message(
      '<style>.hero{color:red}</style><table class="hero" style="width:600px"><tbody><tr><td>Styled mail</td></tr></tbody></table>'
    ));

    expect(html).toContain("<!doctype html>");
    expect(html).toContain("<style>");
    expect(html).toContain(".hero");
    expect(html).toContain('class="hero"');
    expect(html).toContain('style="width:600px"');
  });

  it("removes active content before iframe rendering", () => {
    const html = renderableMailHtml(message(
      '<script>alert(1)</script><img src="https://images.example/a.png" onerror="alert(2)">'
    ));

    expect(html).not.toContain("<script");
    expect(html).not.toContain("onerror");
    expect(html).toContain("https://images.example/a.png");
  });
});
