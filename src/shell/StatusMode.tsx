import { useEffect, useRef, useState } from "react";
import type { ProjectSdk, Snapshot } from "../host/client";
import { TrustControls } from "../TrustControls";
import { ProjectSdkDownload } from "../welcome/ProjectSdkDownload";

// Status-bar trust indicator: a button that opens a small popover to Trust/Restrict the open workspace.
// When the project SDK is missing the popover carries the same block the tree note and the Toolchain
// card render: the bar is one line, so the requirement, the detected versions and the actions live here.
export function StatusMode({ snapshot, onSnapshot, sdkSummary, sdk }: { snapshot: Snapshot; onSnapshot: (snapshot: Snapshot) => void; sdkSummary: string; sdk: { project: ProjectSdk; onOpenGlobalJson: () => void; onCheckAgain: () => void } }) {
  const trusted = snapshot.trust.granted;
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState({ left: 0, bottom: 0 });
  const button = useRef<HTMLButtonElement>(null);
  const popover = useRef<HTMLDivElement>(null);

  function close() {
    setOpen(false);
    button.current?.focus();
  }

  function toggle() {
    const rect = button.current?.getBoundingClientRect();
    // The status bar clips overflow, so the popover is fixed and opens upward from the button.
    if (rect) setAnchor({ left: rect.left, bottom: window.innerHeight - rect.top + 4 });
    setOpen((current) => !current);
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape" && open) {
      event.preventDefault();
      event.stopPropagation();
      close();
    }
  }

  useEffect(() => {
    if (!open) return;
    // Move focus into the popover so the keyboard (and Esc) work without an extra Tab.
    popover.current?.querySelector<HTMLElement>("input, button")?.focus();
    function onPointerDown(event: MouseEvent) {
      const target = event.target as Node;
      if (!popover.current?.contains(target) && !button.current?.contains(target)) setOpen(false);
    }
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  return (
    <>
      <button
        ref={button}
        type="button"
        className={`status-mode ${trusted ? "success" : "warning"}`}
        data-testid="status-mode"
        aria-haspopup="dialog"
        aria-expanded={open}
        title={sdkSummary}
        onClick={toggle}
        onKeyDown={onKeyDown}
      >
        {trusted ? "Trusted mode" : "Restricted mode"}
      </button>
      {open && (
        <div
          ref={popover}
          className="status-mode-popover"
          role="dialog"
          aria-label="Workspace trust"
          data-testid="status-mode-popover"
          style={{ left: anchor.left, bottom: anchor.bottom }}
          onKeyDown={onKeyDown}
        >
          {sdk.project.state === "sdk_missing" ? (
            <div data-testid="status-sdk-block">
              <ProjectSdkDownload
                project={sdk.project}
                surface="status"
                onOpenGlobalJson={sdk.onOpenGlobalJson}
                onCheckAgain={sdk.onCheckAgain}
              />
            </div>
          ) : (
            <p>{sdkSummary}</p>
          )}
          <TrustControls
            snapshot={snapshot}
            onChange={(next) => {
              onSnapshot(next);
              close();
            }}
          />
        </div>
      )}
    </>
  );
}
