import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";
import "./rider-chrome.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

// Load the bundled faces first so xterm measures cells with the real font and nothing reflows.
const faces = ['400 14px "Inter Variable"', '400 13px "JetBrains Mono Variable"'];
Promise.all(faces.map((face) => document.fonts.load(face)))
  .catch(() => undefined)
  .finally(() => createRoot(root).render(<App />));
