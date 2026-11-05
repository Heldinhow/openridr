import { describe, expect, test } from "bun:test";
import { installNativeContextMenuGuard, keepsNativeContextMenu } from "../../src/nativeMenu";

// bun test has no DOM, so the guard is exercised against a tiny element chain that answers `closest`
// the way a browser would for the selector forms the guard uses: tag, `.class`, `[attr]`,
// `[attr="value"]` and one trailing `:not(...)`. The tests below describe DOM semantics
// ("a canvas inside .xterm", "contenteditable=false"), not the guard's selector string.
type Spec = { tag?: string; classes?: string[]; attrs?: Record<string, string> };

type MockElement = {
  tag: string;
  classes: string[];
  attrs: Record<string, string>;
  parent: MockElement | null;
  closest: (selector: string) => Element | null;
  getAttribute: (name: string) => string | null;
};

function mockElement(spec: Spec = {}, parent: MockElement | null = null): MockElement {
  const element: MockElement = {
    tag: spec.tag ?? "div",
    classes: spec.classes ?? [],
    attrs: spec.attrs ?? {},
    parent,
    closest(selectorList: string) {
      for (let node: MockElement | null = element; node; node = node.parent) {
        const matched = selectorList
          .split(",")
          .some((raw) => matchesSelector(raw.trim(), node));
        if (matched) return node as unknown as Element;
      }
      return null;
    },
    getAttribute: (name: string) => (name in element.attrs ? element.attrs[name] : null),
  };
  return element;
}

function matchesSelector(selector: string, node: MockElement): boolean {
  const notAt = selector.indexOf(":not(");
  const base = notAt === -1 ? selector : selector.slice(0, notAt);
  if (!matchesSimple(base, node)) return false;
  if (notAt === -1) return true;
  const inner = selector.slice(notAt + 5, selector.lastIndexOf(")"));
  return !matchesSimple(inner, node);
}

function matchesSimple(selector: string, node: MockElement): boolean {
  const attribute = selector.match(/^\[([\w-]+)(?:="([^"]*)")?\]$/);
  if (attribute) {
    const [, name, value] = attribute;
    if (!(name in node.attrs)) return false;
    return value === undefined || node.attrs[name] === value;
  }
  if (selector.startsWith(".")) return node.classes.includes(selector.slice(1));
  return selector.toLowerCase() === node.tag;
}

function target(element: MockElement): EventTarget {
  return element as unknown as EventTarget;
}

describe("keepsNativeContextMenu", () => {
  test("the terminal keeps the native menu: Copiar/Colar has no app menu equivalent", () => {
    const terminal = mockElement({ classes: ["xterm"] });
    const screen = mockElement({ tag: "canvas", classes: ["xterm-screen"] }, terminal);
    expect(keepsNativeContextMenu(target(screen))).toBe(true);
    expect(keepsNativeContextMenu(target(terminal))).toBe(true);
  });

  test("text entry keeps it, contenteditable=false does not", () => {
    expect(keepsNativeContextMenu(target(mockElement({ tag: "input" })))).toBe(true);
    expect(keepsNativeContextMenu(target(mockElement({ tag: "textarea" })))).toBe(true);
    expect(keepsNativeContextMenu(target(mockElement({ tag: "div", attrs: { contenteditable: "" } })))).toBe(
      true,
    );
    expect(
      keepsNativeContextMenu(target(mockElement({ tag: "span" }, mockElement({ attrs: { contenteditable: "true" } })))),
    ).toBe(true);
    expect(
      keepsNativeContextMenu(target(mockElement({ tag: "div", attrs: { contenteditable: "false" } }))),
    ).toBe(false);
  });

  test("everything else is cancelled", () => {
    expect(keepsNativeContextMenu(target(mockElement({ classes: ["tree-row"] })))).toBe(false);
    expect(keepsNativeContextMenu(null)).toBe(false);
    expect(keepsNativeContextMenu({} as unknown as EventTarget)).toBe(false);
  });
});

describe("installNativeContextMenuGuard", () => {
  function fakeDocument() {
    const listeners: Array<(event: { target: EventTarget | null; preventDefault: () => void }) => void> = [];
    const doc = {
      addEventListener: (type: string, listener: (typeof listeners)[number]) => {
        if (type === "contextmenu") listeners.push(listener);
      },
    };
    const fire = (eventTarget: EventTarget | null) => {
      let prevented = false;
      const event = { target: eventTarget, preventDefault: () => { prevented = true; } };
      for (const listener of listeners) listener(event);
      return prevented;
    };
    return { doc: doc as unknown as Document, fire, listenerCount: () => listeners.length };
  }

  test("prevents the WebKit menu everywhere but text entry and the terminal", () => {
    const { doc, fire } = fakeDocument();
    installNativeContextMenuGuard(doc, true);
    expect(fire(target(mockElement()))).toBe(true);
    expect(fire(target(mockElement({ tag: "input" })))).toBe(false);
    expect(fire(target(mockElement({ classes: ["xterm-screen"] }, mockElement({ classes: ["xterm"] }))))).toBe(
      false,
    );
  });

  test("development keeps it for the webview inspector", () => {
    const { doc, fire, listenerCount } = fakeDocument();
    installNativeContextMenuGuard(doc, false);
    expect(listenerCount()).toBe(0);
    expect(fire(target(mockElement()))).toBe(false);
  });
});
