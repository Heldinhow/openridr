import { describe, expect, test } from "bun:test";
import { installNativeContextMenuGuard, keepsNativeContextMenu } from "../../src/nativeMenu";

type Closest = (selector: string) => Element | null;

function target(closest: Closest | null): EventTarget {
  return (closest ? { closest } : {}) as unknown as EventTarget;
}

function textEntry(): EventTarget {
  return target(() => ({}) as Element);
}

function plain(): EventTarget {
  return target(() => null);
}

describe("keepsNativeContextMenu", () => {
  test("only text entry keeps the native edit menu", () => {
    const asked: string[] = [];
    const probe = (selector: string) => {
      asked.push(selector);
      return null;
    };
    expect(keepsNativeContextMenu(target(probe))).toBe(false);
    expect(asked).toEqual(["input, textarea, [contenteditable]"]);
    expect(keepsNativeContextMenu(textEntry())).toBe(true);
    expect(keepsNativeContextMenu(plain())).toBe(false);
  });

  test("a missing target or a non-element target never keeps it", () => {
    expect(keepsNativeContextMenu(null)).toBe(false);
    expect(keepsNativeContextMenu(target(null))).toBe(false);
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

  test("prevents the WebKit menu everywhere but text entry", () => {
    const { doc, fire } = fakeDocument();
    installNativeContextMenuGuard(doc, true);
    expect(fire(plain())).toBe(true);
    expect(fire(textEntry())).toBe(false);
  });

  test("development keeps it for the webview inspector", () => {
    const { doc, fire, listenerCount } = fakeDocument();
    installNativeContextMenuGuard(doc, false);
    expect(listenerCount()).toBe(0);
    expect(fire(plain())).toBe(false);
  });
});
