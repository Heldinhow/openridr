import type { ProjectSdk } from "../host/client";

// Each surface keeps its own contract testids; `block` prefixes the rest so the status popover and
// the tree note can be in the DOM at the same time without colliding.
const testIds = {
  tree: { block: "sdk", download: "eval-sdk-download", openGlobalJson: "eval-open-global-json" },
  toolchain: { block: "sdk", download: "sdk-download", openGlobalJson: "sdk-open-global-json" },
  status: {
    block: "status-sdk",
    download: "status-sdk-download",
    openGlobalJson: "status-sdk-open-global-json",
  },
};

/**
 * What the project requires, what was detected, and the one action that gets the SDK. The host
 * builds the URLs, the label and the message (which already tells "no SDK installed" from "the
 * installed ones do not satisfy"); this only words the requirement. Nothing here installs.
 */
export function ProjectSdkDownload({
  project,
  surface,
  tfms = [],
  onOpenGlobalJson,
  onCheckAgain,
}: {
  project: ProjectSdk;
  surface: "tree" | "toolchain" | "status";
  /** The declared TargetFrameworks, when the surface has them, to name the origin concretely. */
  tfms?: string[];
  onOpenGlobalJson: () => void;
  /** Re-runs the SDK inventory: the user installed the SDK and asks again, nothing watches the disk. */
  onCheckAgain: () => void;
}) {
  const { requirement, download } = project;
  const ids = testIds[surface];
  const origin =
    requirement.source === "global_json"
      ? `${project.globalJsonPath ?? "global.json"} requests ${
          requirement.version ?? "a pinned SDK"
        }${requirement.rollForward ? ` · rollForward ${requirement.rollForward}` : ""}`
      : requirement.line
        ? `The declared TargetFramework${
            tfms.length > 0 ? ` (${tfms.join(", ")})` : ""
          } requires SDK ${requirement.line} or newer`
        : "The project declares no SDK requirement";

  return (
    <>
      <p data-testid={`${ids.block}-requirement`}>{origin}.</p>
      <p className="muted" data-testid={`${ids.block}-detected`}>
        {project.detected.length > 0
          ? `Detected SDKs: ${project.detected.join(", ")}`
          : "No SDK detected"}
      </p>
      {project.message && <p data-testid={`${ids.block}-project-message`}>{project.message}</p>}
      <p>
        <a
          data-testid={ids.download}
          href={download.url}
          target="_blank"
          rel="noreferrer noopener"
        >
          {download.label}
        </a>
        {download.fallbackUrl && (
          <>
            {" · "}
            <a
              data-testid={`${ids.block}-download-fallback`}
              href={download.fallbackUrl}
              target="_blank"
              rel="noreferrer noopener"
            >
              All .NET {requirement.line} downloads
            </a>
          </>
        )}
      </p>
      <p>
        <button type="button" data-testid={`${ids.block}-check-again`} onClick={onCheckAgain}>
          Check again
        </button>
      </p>
      <p className="muted">OpenRidr never installs an SDK and never rewrites global.json or PATH.</p>
      {project.globalJsonPath && (
        <button type="button" data-testid={ids.openGlobalJson} onClick={onOpenGlobalJson}>
          Open global.json
        </button>
      )}
    </>
  );
}
