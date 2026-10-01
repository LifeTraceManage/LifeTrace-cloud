function applyHoverName(element: Element): void {
  if (!(element instanceof HTMLElement)) return;
  if (element.hasAttribute("title")) return;

  const label = element.getAttribute("aria-label")?.trim();
  if (!label) return;

  // Text buttons already communicate their name visually. This fallback is
  // intended for icon-only native buttons that do not use the shared Button.
  if (element.textContent?.trim()) return;

  element.setAttribute("title", label);
}

function scan(root: ParentNode): void {
  root.querySelectorAll<HTMLElement>('button[aria-label]:not([title]), [role="button"][aria-label]:not([title])')
    .forEach(applyHoverName);
}

export function installButtonHoverNames(root: Document | HTMLElement = document): () => void {
  const target = root instanceof Document ? root.documentElement : root;
  scan(root);

  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === "attributes") {
        applyHoverName(mutation.target as Element);
        continue;
      }
      mutation.addedNodes.forEach((node) => {
        if (!(node instanceof Element)) return;
        applyHoverName(node);
        scan(node);
      });
    }
  });

  observer.observe(target, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: ["aria-label"],
  });

  return () => observer.disconnect();
}
