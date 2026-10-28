import type { ReactNode, RefObject } from "react";
import type { BottomToolId } from "./toolWindows";

const panelClasses: Record<BottomToolId, string> = {
  build: "build-output",
  problems: "problems-output",
  debug: "",
  terminal: "terminal-output-panel",
  tests: "tests-output",
};

export function BottomToolWindow({ tabs, active, onSelect, panelRef, children }: {
  tabs: { id: BottomToolId; label: string }[];
  active: BottomToolId;
  onSelect: (id: BottomToolId) => void;
  panelRef: RefObject<HTMLDivElement>;
  children: ReactNode;
}) {
  return (
    <section className={`output ${panelClasses[active]}`} aria-label="Bottom panel">
      <div className="tabs bottom-tabs" role="tablist" aria-label="Output panels" onKeyDown={(event) => {
        if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
        event.preventDefault();
        const index = tabs.findIndex((tab) => tab.id === active);
        const next = event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
        onSelect(tabs[next].id);
        event.currentTarget.querySelectorAll<HTMLButtonElement>("button")[next]?.focus();
      }}>
        {tabs.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={active === tab.id}
            tabIndex={active === tab.id ? 0 : -1}
            id={`output-tab-${tab.id}`}
            aria-controls="output-content"
            className={`tab-label ${active === tab.id ? "active" : ""}`}
            data-testid={`bottom-tab-${tab.id}`}
            onClick={() => onSelect(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" key={active} ref={panelRef} id="output-content" aria-labelledby={`output-tab-${active}`}>
        {children}
      </div>
    </section>
  );
}
