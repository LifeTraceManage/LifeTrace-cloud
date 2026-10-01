import { useEffect, useRef, useState } from "react";

export function measureMailDocumentHeight(document: Document | null): number | null {
  const root = document?.documentElement;
  if (!document || !root) return null;
  return Math.max(
    120,
    root.scrollHeight,
    document.body?.scrollHeight ?? 0,
  );
}

export function MailHtmlFrame({
  html,
  title,
}: {
  html: string;
  title: string;
}) {
  const ref = useRef<HTMLIFrameElement>(null);
  const [height, setHeight] = useState(240);

  useEffect(() => {
    const frame = ref.current;
    if (!frame) return;

    let observer: ResizeObserver | null = null;
    let animationFrame = 0;

    const syncHeight = () => {
      cancelAnimationFrame(animationFrame);
      animationFrame = requestAnimationFrame(() => {
        const nextHeight = measureMailDocumentHeight(frame.contentDocument);
        if (nextHeight !== null) setHeight(nextHeight);
      });
    };

    const attach = () => {
      observer?.disconnect();
      const document = frame.contentDocument;
      const root = document?.documentElement;
      if (!document || !root) return;

      syncHeight();
      observer = new ResizeObserver(syncHeight);
      observer.observe(root);
      if (document.body) observer.observe(document.body);

      document.querySelectorAll("img").forEach((image) => {
        image.addEventListener("load", syncHeight);
        image.addEventListener("error", syncHeight);
      });
    };

    frame.addEventListener("load", attach);
    if (frame.contentDocument?.readyState === "complete") attach();

    return () => {
      frame.removeEventListener("load", attach);
      observer?.disconnect();
      cancelAnimationFrame(animationFrame);
    };
  }, [html]);

  return (
    <iframe
      ref={ref}
      title={title}
      srcDoc={html}
      sandbox="allow-same-origin allow-popups allow-popups-to-escape-sandbox"
      className="block w-full border-0 bg-white"
      style={{ height }}
    />
  );
}
