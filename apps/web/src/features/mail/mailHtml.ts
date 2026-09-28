import type { MailMessageDetail } from "./types";

function normalizeContentId(value?: string | null): string {
  return (value ?? "")
    .trim()
    .replace(/^<|>$/g, "")
    .toLocaleLowerCase();
}

function decodeCid(value: string): string {
  const raw = value.slice(4).trim().replace(/^<|>$/g, "");
  try {
    return decodeURIComponent(raw).toLocaleLowerCase();
  } catch {
    return raw.toLocaleLowerCase();
  }
}

function safeExternalUrl(value: string): boolean {
  try {
    const url = new URL(value, window.location.origin);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export function renderableMailHtml(message: MailMessageDetail): string {
  if (!message.html || typeof DOMParser === "undefined") return message.html ?? "";

  const document = new DOMParser().parseFromString(message.html, "text/html");
  const inlineByContentId = new Map(
    message.attachments
      .map((attachment) => [normalizeContentId(attachment.contentId), attachment] as const)
      .filter(([contentId]) => Boolean(contentId)),
  );

  // Defense in depth: these should already be removed server-side.
  document.querySelectorAll("script, iframe, object, embed").forEach((element) => element.remove());
  document.querySelectorAll("*").forEach((element) => {
    for (const attribute of Array.from(element.attributes)) {
      if (/^on/i.test(attribute.name)) element.removeAttribute(attribute.name);
    }
  });

  document.querySelectorAll("a[href]").forEach((anchor) => {
    const href = anchor.getAttribute("href")?.trim() ?? "";
    if (!href || !safeExternalUrl(href)) {
      anchor.removeAttribute("href");
      return;
    }
    anchor.setAttribute("target", "_blank");
    anchor.setAttribute("rel", "noopener noreferrer");
    anchor.setAttribute("referrerpolicy", "no-referrer");
  });

  document.querySelectorAll("img").forEach((image) => {
    const src = image.getAttribute("src")?.trim() ?? "";
    if (!src) return;

    if (src.toLocaleLowerCase().startsWith("cid:")) {
      const attachment = inlineByContentId.get(decodeCid(src));
      const inlineUrl = attachment?.downloadUrl?.trim();
      if (inlineUrl) {
        image.setAttribute("src", inlineUrl);
      } else {
        image.removeAttribute("src");
      }
    } else if (!safeExternalUrl(src)) {
      image.removeAttribute("src");
    }

    image.setAttribute("loading", "lazy");
    image.setAttribute("referrerpolicy", "no-referrer");
  });

  const meta = document.createElement("meta");
  meta.setAttribute("name", "viewport");
  meta.setAttribute("content", "width=device-width, initial-scale=1");
  document.head.prepend(meta);

  const frameStyle = document.createElement("style");
  frameStyle.textContent = `
    html, body { margin: 0; padding: 0; max-width: 100%; }
    body { overflow-wrap: anywhere; }
    img { max-width: 100%; height: auto; }
  `;
  document.head.append(frameStyle);

  return "<!doctype html>\n" + document.documentElement.outerHTML;
}
