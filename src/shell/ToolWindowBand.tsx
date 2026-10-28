import { Icon } from "../Icon";
import type { ToolWindow, ToolWindowId, ToolWindowSlot } from "./toolWindows";

export function ToolWindowBand({ slot, items, activeId, open, onToggle }: {
  slot: ToolWindowSlot;
  items: ToolWindow[];
  activeId: ToolWindowId;
  open: boolean;
  onToggle: (id: ToolWindowId) => void;
}) {
  return (
    <div
      className={`toolband toolband-${slot}`}
      role="toolbar"
      aria-label={slot === "left" ? "Left tool windows" : "Bottom tool windows"}
      data-testid={`toolband-${slot}`}
    >
      {items.map((item) => {
        const pressed = open && item.id === activeId;
        return (
          <button
            key={item.id}
            type="button"
            className={`toolband-button ${pressed ? "active" : ""}`}
            data-testid={`toolband-${item.id}`}
            aria-label={item.title}
            aria-pressed={pressed}
            title={item.title}
            onClick={() => onToggle(item.id)}
          >
            <Icon name={item.icon} />
          </button>
        );
      })}
    </div>
  );
}
