import { useState } from "react";

type Props = { initialPath: string; busy: boolean; invalid: boolean; onOpen: (path: string) => void };

// Browser/E2E bridge only: Playwright cannot drive the native chooser, so it types the path.
export function PathForm({ initialPath, busy, invalid, onOpen }: Props) {
  const [path, setPath] = useState(initialPath);
  return (
    <form
      className="path-form"
      onSubmit={(event) => {
        event.preventDefault();
        onOpen(path);
      }}
    >
      <label htmlFor="workspace-path-input" className="sr-only">Workspace path (absolute)</label>
      <div className="open-row">
        <input
          id="workspace-path-input"
          data-testid="workspace-path-input"
          value={path}
          onChange={(event) => setPath(event.target.value)}
          placeholder="Path to a .sln, .csproj or folder"
          aria-invalid={invalid ? true : undefined}
          spellCheck={false}
          disabled={busy}
        />
        <button type="submit" className="welcome-button" data-testid="workspace-open" disabled={busy}>Open path</button>
      </div>
    </form>
  );
}
