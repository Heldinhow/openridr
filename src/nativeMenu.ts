// A WebKit/WKWebView contextmenu event that nobody handles ends in the browser's own menu:
// "Reload", "Share…", "Services". In a shipped IDE that menu is a browser affordance, never a
// product one, so production prevents it everywhere except text entry, where the OS edit menu
// (cut/copy/paste, spelling, substitutions) is still what a user expects.

const TEXT_ENTRY = "input, textarea, [contenteditable]";

/** True when the target is text entry, i.e. the native edit menu may stay. */
export function keepsNativeContextMenu(target: EventTarget | null): boolean {
  const element = target as Element | null;
  if (!element || typeof element.closest !== "function") return false;
  return element.closest(TEXT_ENTRY) !== null;
}

/**
 * Install the app-wide guard. `enabled` is false while developing, where the WebKit menu
 * (Reload, Inspect Element) is a debugging tool rather than a defect.
 */
export function installNativeContextMenuGuard(
  doc: Pick<Document, "addEventListener"> = document,
  enabled = true,
): void {
  if (!enabled) return;
  doc.addEventListener("contextmenu", (event) => {
    if (!keepsNativeContextMenu(event.target)) event.preventDefault();
  });
}
