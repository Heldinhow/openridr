import "./does-not-exist.png";
import { createRoot } from "react-dom/client";
import App from "./App";
import { installNativeContextMenuGuard } from "./nativeMenu";
import "./styles.css";
import "./rider-chrome.css";
import "./git-panel.css";
import "./solution-icons.css";

// The WebKit menu (Reload / Share… / Services) belongs to a browser, not to the shipped app.
// Development keeps it: right-click → Inspect Element is how the webview gets debugged.
installNativeContextMenuGuard(document, import.meta.env.PROD);

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

// Load the bundled faces first so xterm measures cells with the real font and nothing reflows.
const faces = ['400 14px "Inter Variable"', '400 13px "JetBrains Mono Variable"'];
Promise.all(faces.map((face) => document.fonts.load(face)))
  .catch(() => undefined)
  .finally(() => createRoot(root).render(<App />));
