import { useEffect, useRef, useState } from "react";
import { call, HostCallError, type Snapshot, type Terminal } from "./host/client";

type Props = {
  snapshot: Snapshot;
  disabledReason: string | null;
  onSnapshot: (snapshot: Snapshot) => void;
  onError: (message: string) => void;
};

const stateLabels: Record<Terminal["state"], string> = {
  idle: "Not started",
  running: "Running",
  exited: "Exited",
  stopped: "Stopped",
  failed: "Failed",
};

function messageOf(error: unknown): string {
  if (error instanceof HostCallError || error instanceof Error) return error.message;
  return String(error);
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function charWidth(style: CSSStyleDeclaration) {
  const context = document.createElement("canvas").getContext("2d");
  if (!context) return 8;
  context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
  const width = context.measureText("MMMMMMMMMM").width / 10;
  return width > 0 ? width : 8;
}

/** One real PTY shell: state, identity, scrollable output, a line input and Ctrl+C. */
export function TerminalPanel({ snapshot, disabledReason, onSnapshot, onError }: Props) {
  const terminal = snapshot.terminal;
  const running = terminal.state === "running";
  const [line, setLine] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [confirmClose, setConfirmClose] = useState(false);
  const outputRef = useRef<HTMLPreElement>(null);
  const sentSize = useRef("");

  async function invoke(label: string, method: string, params: Record<string, unknown> = {}) {
    setBusy(label);
    try {
      onSnapshot(await call<Snapshot>(method, params));
    } catch (error) {
      onError(`${label} failed: ${messageOf(error)}`);
    } finally {
      setBusy(null);
    }
  }

  useEffect(() => {
    const output = outputRef.current;
    if (output) output.scrollTop = output.scrollHeight;
  }, [terminal.log]);

  // Report the real panel size so the shell wraps like the visible terminal, not a guess.
  useEffect(() => {
    if (!running) return;
    const output = outputRef.current;
    if (!output) return;
    let timer = 0;
    const fit = () => {
      const rect = output.getBoundingClientRect();
      const style = window.getComputedStyle(output);
      const cols = clamp(Math.floor((rect.width - 24) / charWidth(style)), 20, 400);
      const lineHeight = Number.parseFloat(style.lineHeight) || 16;
      const rows = clamp(Math.floor((rect.height - 12) / lineHeight), 5, 200);
      const key = `${cols}x${rows}`;
      if (sentSize.current === key) return;
      sentSize.current = key;
      call<Snapshot>("terminal.resize", { cols, rows })
        .then(onSnapshot)
        .catch(() => {
          sentSize.current = "";
        });
    };
    const observer = new ResizeObserver(() => {
      window.clearTimeout(timer);
      timer = window.setTimeout(fit, 250);
    });
    observer.observe(output);
    timer = window.setTimeout(fit, 50);
    return () => {
      observer.disconnect();
      window.clearTimeout(timer);
    };
  }, [running]);

  async function send(data: string) {
    if (!running) return;
    await invoke("Terminal input", "terminal.write", { data });
  }

  function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!running) return;
    const text = line;
    setLine("");
    void send(`${text}\r`);
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === "c") {
      // Ctrl+C belongs to the shell: send 0x03 instead of copying anything.
      event.preventDefault();
      void send("\u0003");
    }
  }

  const exitLabel = terminal.exit === null ? "exit unknown" : String(terminal.exit);

  return (
    <section className="terminal-panel" aria-label="Terminal" data-testid="terminal-panel">
      {disabledReason && (
        <p className="muted" data-testid="terminal-disabled-reason">
          {disabledReason}
        </p>
      )}
      <div className="terminal-meta">
        <span>
          Shell: <strong data-testid="terminal-shell">{terminal.shell || "not started"}</strong>
        </span>
        <span>
          Cwd: <span data-testid="terminal-cwd">{terminal.cwd || "not started"}</span>
        </span>
        <span data-testid="terminal-size">
          {terminal.cols}×{terminal.rows}
        </span>
        <span className={`tag ${terminal.state === "failed" ? "error" : ""}`} data-testid="terminal-state" role="status">
          {stateLabels[terminal.state]}
        </span>
        <span className="muted" data-testid="terminal-process">
          {terminal.processId === null ? "No shell process" : `Process ${terminal.processId}`}
        </span>
      </div>

      <div className="terminal-controls">
        <button
          data-testid="terminal-start"
          disabled={disabledReason !== null || running || busy !== null}
          onClick={() => void invoke("Start terminal", "terminal.start")}
          title={
            disabledReason ??
            "Starts the discovered shell in the workspace root; the shell and cwd are shown above."
          }
        >
          Start shell
        </button>
        {running && (
          <button
            data-testid="terminal-close"
            disabled={busy !== null}
            onClick={() => setConfirmClose(true)}
            title="Closing a running shell needs a decision: keep the terminal or terminate the shell."
          >
            Close…
          </button>
        )}
        <button
          data-testid="terminal-interrupt"
          disabled={!running || busy !== null}
          onClick={() => void send("\u0003")}
          title="Sends Ctrl+C (byte 0x03) to the PTY so the foreground child receives SIGINT."
        >
          Ctrl+C
        </button>
      </div>

      {confirmClose && (
        <div
          className="terminal-confirm"
          role="dialog"
          aria-label="Close the terminal"
          data-testid="terminal-confirm"
          onKeyDown={(event) => {
            if (event.key === "Escape") setConfirmClose(false);
          }}
        >
          <p>
            The shell is still running. Keep the terminal, or terminate the shell and close it? The
            log stays visible either way.
          </p>
          <button data-testid="terminal-confirm-keep" autoFocus onClick={() => setConfirmClose(false)}>
            Keep terminal
          </button>
          <button
            data-testid="terminal-confirm-stop"
            onClick={() => {
              setConfirmClose(false);
              void invoke("Close terminal", "terminal.stop", { confirm: true });
            }}
          >
            Terminate shell and close
          </button>
        </div>
      )}

      {terminal.sealed && (
        <p className="muted" data-testid="terminal-sealed">
          {terminal.staleReason ?? "The shell record was sealed by a workspace boundary."}
        </p>
      )}
      {terminal.logTruncated && (
        <p className="muted" data-testid="terminal-log-truncated">
          Output is bounded; earlier lines are not kept.
        </p>
      )}
      {running && (
        <p className="muted" data-testid="terminal-hint">
          Type a line and press Enter. Ctrl+C while this input is focused interrupts the foreground
          child; it never copies text.
        </p>
      )}

      <pre
        className="terminal-output"
        data-testid="terminal-output"
        aria-label="Terminal output"
        tabIndex={0}
        ref={outputRef}
      >
        {terminal.log || "No output yet."}
      </pre>

      {terminal.revision > 0 && !running && (
        <div className="terminal-result">
          <span data-testid="terminal-exit">Exit: {exitLabel}</span>
          {terminal.termination && (
            <span data-testid="terminal-termination">
              Termination requested: {terminal.termination.requested} · Observed:{" "}
              {terminal.termination.observed}
              {terminal.termination.killedAfterGrace ? " · killed after grace" : ""}
            </span>
          )}
        </div>
      )}

      <form className="terminal-input-row" onSubmit={submit}>
        <input
          data-testid="terminal-input"
          aria-label="Terminal input"
          placeholder={running ? "Type a command and press Enter" : "Start the shell to type"}
          value={line}
          onChange={(event) => setLine(event.target.value)}
          onKeyDown={onKeyDown}
          disabled={!running}
          spellCheck={false}
          autoComplete="off"
        />
        <button type="submit" data-testid="terminal-send" disabled={!running || busy !== null}>
          Send line
        </button>
      </form>
    </section>
  );
}
