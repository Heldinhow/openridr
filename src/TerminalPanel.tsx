import { Icon } from "./Icon";
import { useEffect, useRef, useState } from "react";
import { Terminal as XtermTerminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { call, HostCallError, type Snapshot, type Terminal } from "./host/client";

type Props = {
  snapshot: Snapshot;
  disabledReason: string | null;
  onSnapshot: (snapshot: Snapshot) => void;
  onError: (message: string) => void;
};

type RawRead = { revision: number; chunk: { from: number; end: number; truncated: boolean; bytes: number[] } };

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

/** One real PTY shell rendered by xterm: typing goes straight to the PTY; the line form is a mirror. */
export function TerminalPanel({ snapshot, disabledReason, onSnapshot, onError }: Props) {
  const terminal = snapshot.terminal;
  const running = terminal.state === "running";
  const [line, setLine] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [confirmClose, setConfirmClose] = useState(false);
  const screenRef = useRef<HTMLDivElement>(null);
  const xterm = useRef<XtermTerminal | null>(null);
  const fit = useRef<FitAddon | null>(null);
  // Byte offset of the next read and the session it belongs to; a new revision starts from zero.
  const readPosition = useRef({ revision: -1, since: 0 });
  const sentSize = useRef("");
  const resize = useRef<() => void>(() => {});
  // Keystrokes reach the PTY in order, one call at a time.
  const inputQueue = useRef<Promise<unknown>>(Promise.resolve());
  const latest = useRef({ running, onSnapshot, onError });
  latest.current = { running, onSnapshot, onError };

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

  // The emulator lives as long as the panel; its size follows the panel, and the shell learns it.
  useEffect(() => {
    const screen = screenRef.current;
    if (!screen) return;
    const term = new XtermTerminal({
      convertEol: false,
      cursorBlink: true,
      fontFamily: "Menlo, Monaco, 'Courier New', monospace",
      fontSize: 12,
      scrollback: 10_000,
      theme: { background: "#000000", foreground: "#f5f5f5", selectionBackground: "#1e3358" },
    });
    const addon = new FitAddon();
    term.loadAddon(addon);
    term.open(screen);
    xterm.current = term;
    fit.current = addon;
    const sendSize = () => {
      addon.fit();
      const key = `${term.cols}x${term.rows}`;
      if (sentSize.current === key || !latest.current.running) return;
      sentSize.current = key;
      call<Snapshot>("terminal.resize", { cols: term.cols, rows: term.rows })
        .then((next) => latest.current.onSnapshot(next))
        .catch(() => {
          sentSize.current = "";
        });
    };
    resize.current = sendSize;
    const observer = new ResizeObserver(() => sendSize());
    observer.observe(screen);
    const input = term.onData((data) => {
      if (!latest.current.running) return;
      inputQueue.current = inputQueue.current.then(() =>
        call<Snapshot>("terminal.write", { data })
          .then((next) => latest.current.onSnapshot(next))
          .catch((error) => latest.current.onError(`Terminal input failed: ${messageOf(error)}`)),
      );
    });
    return () => {
      input.dispose();
      observer.disconnect();
      term.dispose();
      xterm.current = null;
      fit.current = null;
      readPosition.current = { revision: -1, since: 0 };
      sentSize.current = "";
    };
  }, []);

  // The shell learns the screen size as soon as it runs, even when the panel did not resize in between.
  useEffect(() => {
    if (running) resize.current();
  }, [running]);

  // Raw PTY bytes arrive in order; the poll is quick while running and slow once the shell has ended.
  useEffect(() => {
    let stopped = false;
    let timer = 0;
    const pump = async () => {
      const term = xterm.current;
      if (!stopped && term && terminal.revision > 0) {
        try {
          const read = await call<RawRead>("terminal.read", { since: readPosition.current.since });
          if (read.revision !== readPosition.current.revision) {
            term.reset();
            readPosition.current = { revision: read.revision, since: 0 };
            sentSize.current = "";
            fit.current?.fit();
            // Reading again from zero is the only way to get the output of this session.
            const again = await call<RawRead>("terminal.read", { since: 0 });
            if (again.chunk.bytes.length > 0) term.write(Uint8Array.from(again.chunk.bytes));
            readPosition.current.since = again.chunk.end;
          } else if (read.chunk.bytes.length > 0) {
            term.write(Uint8Array.from(read.chunk.bytes));
            readPosition.current.since = read.chunk.end;
          }
        } catch {
          // A failed poll is retried on the next tick; the shell state itself is reported by the host.
        }
      }
      if (!stopped) timer = window.setTimeout(pump, latest.current.running ? 30 : 250);
    };
    timer = window.setTimeout(pump, 0);
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, [terminal.revision]);

  // Trusted workspaces open a shell with a prompt by themselves; Restricted never starts one.
  const autoStarted = useRef<number | null>(null);
  useEffect(() => {
    if (disabledReason !== null || terminal.state !== "idle" || autoStarted.current === snapshot.contextId) return;
    autoStarted.current = snapshot.contextId;
    void invoke("Start terminal", "terminal.start");
    // invoke is recreated each render; the guard above keeps this to once per context.
  }, [disabledReason, terminal.state, snapshot.contextId]);

  async function send(data: string) {
    if (!running) return;
    await invoke("Terminal input", "terminal.write", { data });
  }

  // The line form mirrors what the screen takes: one line plus Enter, for screen readers and IME.
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
      <div className="terminal-toolbar">
        <div className="terminal-meta" role="region" tabIndex={0} aria-label="Terminal details">
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
          <span className="sr-only" data-testid="terminal-process">
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
            <Icon name="run" /><span className="sr-only">Start shell</span>
          </button>
          {running && (
            <button
              data-testid="terminal-close"
              disabled={busy !== null}
              onClick={() => setConfirmClose(true)}
              title="Closing a running shell needs a decision: keep the terminal or terminate the shell."
            >
              <Icon name="stop" /><span className="sr-only">Close…</span>
            </button>
          )}
          <button
            data-testid="terminal-interrupt"
            disabled={!running || busy !== null}
            onClick={() => void send("\u0003")}
            title="Sends Ctrl+C (byte 0x03) to the PTY so the foreground child receives SIGINT."
          >
            <kbd>^C</kbd><span className="sr-only">Ctrl+C</span>
          </button>
        </div>
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
          <button className="primary" data-testid="terminal-confirm-keep" autoFocus onClick={() => setConfirmClose(false)}>
            Keep terminal
          </button>
          <button
            data-testid="terminal-confirm-stop"
            onClick={() => {
              setConfirmClose(false);
              void invoke("Close terminal", "terminal.stop", { confirm: true });
            }}
          >
            <Icon name="problems" /> Terminate shell and close
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
        <p className="sr-only" data-testid="terminal-hint">
          Type in the terminal as in any shell. The line input below mirrors it for assistive tech.
        </p>
      )}

      <div
        className="terminal-output terminal-screen"
        data-testid="terminal-output"
        aria-label="Terminal output"
        ref={screenRef}
      />

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
        <button type="submit" className="primary" data-testid="terminal-send" disabled={!running || busy !== null}>
          Send line
        </button>
      </form>
    </section>
  );
}
