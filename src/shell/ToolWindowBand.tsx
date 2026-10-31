import { Icon } from "../Icon";
import type { ResolvedAction } from "../actions/registry";
import type { ToolWindow, ToolWindowId, ToolWindowSlot } from "./toolWindows";

export function ToolWindowBand({ slot, items, actions, activeId, open }: {
  slot: ToolWindowSlot;
  items: ToolWindow[];
  actions: ResolvedAction[];
  activeId: ToolWindowId;
  open: boolean;
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
        const action = actions.find((candidate) => candidate.id === `toolWindow.${item.id}`);
        return (
          <button
            key={item.id}
            type="button"
            className={`toolband-button ${pressed ? "active" : ""}`}
            data-testid={`toolband-${item.id}`}
            aria-label={action?.title ?? item.title}
            aria-pressed={pressed}
            title={action?.title ?? item.title}
            onClick={() => action?.execute()}
          >
            <Icon name={item.icon} />
          </button>
        );
      })}
    </div>
  );
}
