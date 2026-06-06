import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect } from "react";

// Battery: open external http(s) links in the system browser instead of inside
// the Tauri webview. Capture-phase listener so it runs before React handlers.
export function useExternalLinks() {
  useEffect(() => {
    const onClick = (e: MouseEvent) => {
      const anchor = (e.target as HTMLElement | null)?.closest("a");
      const href = anchor?.getAttribute("href");
      if (!href || !/^https?:\/\//i.test(href)) return;
      if (href.startsWith(window.location.origin)) return;
      e.preventDefault();
      void openUrl(href);
    };
    document.addEventListener("click", onClick, true);
    return () => document.removeEventListener("click", onClick, true);
  }, []);
}
