import { useEffect, useRef, useState } from "react";

type Props = {
  initialPath: string;
  busy: boolean;
  invalid: boolean;
  // Why the native chooser is not being used. A status line, never an error: the field is the way out.
  notice?: string;
  // Bumped by a refused chooser so the field takes focus without being remounted.
  focusRequest?: number;
  onOpen: (path: string) => void;
};

// The typed path is the fallback the welcome screen always keeps: not every platform has a native
// chooser, and Playwright cannot drive one either.
export function PathForm({ initialPath, busy, invalid, notice, focusRequest = 0, onOpen }: Props) {
  const [path, setPath] = useState(initialPath);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (focusRequest > 0) input.current?.focus();
  }, [focusRequest]);

  return (
    <form
      className="path-form"
      onSubmit={(event) => {
        event.preventDefault();
        onOpen(path);
      }}
    >
      {notice && (
        <p className="path-notice muted" role="status" data-testid="workspace-pick-notice">{notice}</p>
      )}
      <label htmlFor="workspace-path-input" className="sr-only">Workspace path</label>
      <div className="open-row">
        <input
          ref={input}
          id="workspace-path-input"
          data-testid="workspace-path-input"
          value={path}
          onChange={(event) => setPath(event.target.value)}
          placeholder="Path to a .sln, .slnx, .csproj or folder (~/src/App.sln)"
          aria-invalid={invalid ? true : undefined}
          spellCheck={false}
          disabled={busy}
        />
        <button type="submit" className="welcome-button" data-testid="workspace-open" disabled={busy}>Open path</button>
      </div>
    </form>
  );
}
