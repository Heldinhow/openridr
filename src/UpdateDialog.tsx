import { useCallback, useEffect, useRef, useState } from "react";
import { messageOf } from "./host/client";

/** What `update_check` answers; mirrors `CheckResult` in the shell. */
type CheckResult = {
  state: "available" | "current" | "unsupported";
  version: string | null;
  current: string | null;
  notes: string | null;
};

/** Payload of `openridr-update-progress`. */
type Progress = { downloaded: number; total: number | null };

type View =
  | { kind: "closed" }
  | { kind: "checking" }
  | { kind: "available"; update: CheckResult }
  | { kind: "downloading"; update: CheckResult; progress: Progress | null }
  | { kind: "installed"; update: CheckResult }
  | { kind: "failed"; update: CheckResult; message: string }
  | { kind: "notice"; message: string };

/** How long after the first paint the silent check runs; long enough not to compete with startup. */
const STARTUP_CHECK_DELAY = 3000;

function megabytes(bytes: number) {
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

/**
 * The in-app update: a silent check at startup, the native "Check for Updates…" item, the
 * confirmation, the download bar and the restart. Only the desktop shell has the updater; the dev
 * HTTP bridge renders nothing.
 */
export function UpdateDialog({ onRestart }: { onRestart: () => void }) {
  const [view, setView] = useState<View>({ kind: "closed" });
  // One update operation at a time: a second check would replace the pending update the user is
  // looking at, and a check during the download would hide the progress that proves it is running.
  const busy = useRef(false);

  const check = useCallback(async (manual: boolean) => {
    if (busy.current) return;
    busy.current = true;
    // The startup check stays invisible until it has something to say; the menu one shows itself.
    if (manual) setView({ kind: "checking" });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<CheckResult>("update_check");
      if (result.state === "available") {
        setView({ kind: "available", update: result });
      } else if (manual) {
        setView({
          kind: "notice",
          message:
            result.state === "current"
              ? `OpenRidr ${result.current} is the newest version.`
              : "This build does not update itself; install OpenRidr from the release DMG.",
        });
      }
    } catch (error) {
      // A check that fails at startup is a log line in the shell, not a dialog nobody asked for.
      if (manual) setView({ kind: "notice", message: messageOf(error) });
    } finally {
      busy.current = false;
    }
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    let stop: (() => void) | undefined;
    let timer: number | undefined;
    let unmounted = false;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const stopCheck = await listen("openridr-update-check", () => void check(true));
      const stopProgress = await listen<Progress>("openridr-update-progress", (event) => {
        setView((current) =>
          current.kind === "downloading" ? { ...current, progress: event.payload } : current,
        );
      });
      if (unmounted) {
        stopCheck();
        stopProgress();
        return;
      }
      stop = () => {
        stopCheck();
        stopProgress();
      };
      timer = window.setTimeout(() => void check(false), STARTUP_CHECK_DELAY);
    })().catch(() => {});
    return () => {
      unmounted = true;
      window.clearTimeout(timer);
      stop?.();
    };
  }, [check]);

  async function install(update: CheckResult) {
    busy.current = true;
    setView({ kind: "downloading", update, progress: null });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      // The shell installs exactly the version this dialog is showing.
      await invoke("update_install", { version: update.version });
      setView({ kind: "installed", update });
    } catch (error) {
      setView({ kind: "failed", update, message: messageOf(error) });
      busy.current = false;
    }
  }

  function close() {
    busy.current = false;
    setView({ kind: "closed" });
  }

  if (view.kind === "closed") return null;

  const downloading = view.kind === "downloading";
  const update = "update" in view ? view.update : null;
  const progress = downloading ? view.progress : null;
  const total = progress?.total ?? null;
  const downloaded = progress?.downloaded ?? 0;
  // The last progress event fires when the body is complete; from there the shell is verifying the
  // signature and swapping the bundle, which has no progress of its own.
  const installing = total !== null && downloaded >= total;

  return (
    <div className="modal-backdrop">
      <div
        className="modal update-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-title"
        data-testid="update-dialog"
        onKeyDown={(event) => {
          // Escape during the download would hide the only sign that it is still running.
          if (event.key === "Escape" && !downloading) {
            event.preventDefault();
            close();
          }
        }}
      >
        <h2 id="update-title">
          {view.kind === "notice" ? "Check for Updates" : "Update OpenRidr"}
        </h2>

        {view.kind === "checking" && <p className="muted">Checking for updates…</p>}

        {view.kind === "notice" && (
          <p data-testid="update-check-result">{view.message}</p>
        )}

        {update && (
          <p data-testid="update-version">
            OpenRidr <strong>{update.version}</strong> is available; you have {update.current}.
          </p>
        )}

        {update?.notes && (
          <pre className="update-notes" data-testid="update-notes">
            {update.notes}
          </pre>
        )}

        {downloading &&
          (installing ? (
            <p className="muted" data-testid="update-progress">
              Installing…
            </p>
          ) : (
            <>
              <progress
                data-testid="update-progress"
                max={total ?? undefined}
                value={total === null ? undefined : downloaded}
              />
              <p className="muted">
                {total === null
                  ? `Downloading… ${megabytes(downloaded)}`
                  : `Downloading… ${Math.floor((downloaded / total) * 100)}%`}
              </p>
            </>
          ))}

        {view.kind === "installed" && (
          <p>
            OpenRidr {view.update.version} is installed. Restart to start using it; unsaved files
            are handled before the app closes.
          </p>
        )}

        {view.kind === "failed" && (
          <p className="error" data-testid="update-error">
            {view.message}
          </p>
        )}

        <div className="actions">
          {view.kind === "available" && (
            <>
              <button type="button" data-testid="update-later" onClick={close}>
                Later
              </button>
              <button
                type="button"
                className="primary"
                data-testid="update-install"
                onClick={() => void install(view.update)}
              >
                Download and install
              </button>
            </>
          )}
          {view.kind === "installed" && (
            <>
              <button type="button" data-testid="update-later" onClick={close}>
                Later
              </button>
              <button
                type="button"
                className="primary"
                data-testid="update-restart"
                onClick={() => {
                  close();
                  onRestart();
                }}
              >
                Restart now
              </button>
            </>
          )}
          {view.kind === "failed" && (
            <>
              <button type="button" onClick={close}>
                Close
              </button>
              <button type="button" className="primary" onClick={() => void check(true)}>
                Try again
              </button>
            </>
          )}
          {(view.kind === "notice" || view.kind === "checking") && (
            <button type="button" onClick={close}>
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
