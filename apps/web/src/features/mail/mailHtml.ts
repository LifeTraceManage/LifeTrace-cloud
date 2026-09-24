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

export function renderableMailHtml(message: MailMessageDetail): string {
  if (!message.html || typeof DOMParser === "undefined") return message.html ?? "";

  const document = new DOMParser().parseFromString(message.html, "text/html");
  const inlineByContentId = new Map(
    message.attachments
      .map((attachment) => [normalizeContentId(attachment.contentId), attachment] as const)
      .filter(([contentId]) => Boolean(contentId)),
  );

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
    }

    image.setAttribute("loading", "lazy");
    image.setAttribute("referrerpolicy", "no-referrer");
    image.style.maxWidth = "100%";
    image.style.height = "auto";
  });

  return document.body.innerHTML;
}
