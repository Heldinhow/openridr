// A WebKit/WKWebView contextmenu event that nobody handles ends in the browser's own menu:
// "Reload", "Share…", "Services". In a shipped IDE that menu is a browser affordance, never a
// product one, so production prevents it everywhere except where the OS menu is the real feature:
//
// - text entry (input/textarea/contenteditable), where cut/copy/paste and spelling live;
// - the terminal (`.xterm`), whose own contextmenu handler only puts the focus on its hidden
//   textarea and leaves the menu (Copy/Paste, i.e. Copiar/Colar) to the system. Cancelling it there
//   would remove the only way to copy out of a shell.
// `contenteditable="false"` is not editable and stays cancelled.
const NATIVE_MENU_TARGETS = 'input, textarea, .xterm, [contenteditable]:not([contenteditable="false"])';

/** True when the target keeps the native OS menu. */
export function keepsNativeContextMenu(target: EventTarget | null): boolean {
  const element = target as Element | null;
  if (!element || typeof element.closest !== "function") return false;
  return element.closest(NATIVE_MENU_TARGETS) !== null;
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
