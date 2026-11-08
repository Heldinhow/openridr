import type { ProjectSdk } from "../host/client";
import { ProjectSdkDownload } from "./ProjectSdkDownload";

export function ProjectSdkBody({
  project,
  tfms,
  onOpenGlobalJson,
  onCheckAgain,
}: {
  project: ProjectSdk;
  tfms: string[];
  onOpenGlobalJson: () => void;
  onCheckAgain: () => void;
}) {
  const installLink = (
    <a href={project.instructionsUrl} target="_blank" rel="noreferrer noopener">
      Official .NET SDK downloads
    </a>
  );
  const origin = project.globalJsonPath ?? "global.json";
  const pin = project.requestedVersion
    ? `${origin} requests ${project.requestedVersion} · rollForward ${
        project.rollForward ?? "patch"
      }`
    : null;
  const declared = tfms.length > 0 ? tfms.join(", ") : null;
  // The host decides the requirement; this only words it. A project with no pin is not a
  // problem: the TargetFramework is the requirement.
  const requirement =
    project.requirement.source === "target_framework"
      ? `Not pinned: the declared TargetFramework${
          declared ? ` (${declared})` : ""
        } requires SDK ${project.requirement.line} or newer`
      : project.requirement.source === "none"
        ? "Not pinned and no TargetFramework requirement: the highest installed stable SDK is used"
        : null;
  const openGlobalJson = project.globalJsonPath ? (
    <button type="button" data-testid="sdk-open-global-json" onClick={onOpenGlobalJson}>
      Open global.json
    </button>
  ) : null;

  switch (project.state) {
    case "resolved":
      return (
        <>
          <strong className="success">SDK {project.resolvedVersion}</strong>{" "}
          <span className="tag">resolved after trust · not a build</span>
          <p>
            {pin ?? requirement}
            {project.allowPrerelease ? " · prerelease allowed" : ""}.
          </p>
          {project.message && <p data-testid="sdk-project-message">{project.message}</p>}
          <p className="muted">
            Resolved root: <code>{project.root}</code>
            <br />
            SDK path: <code>{project.sdkPath}</code>
          </p>
        </>
      );
    case "sdk_missing":
      return (
        <>
          <strong className="error">Project SDK missing</strong>
          <ProjectSdkDownload
            project={project}
            surface="toolchain"
            tfms={tfms}
            onOpenGlobalJson={onOpenGlobalJson}
            onCheckAgain={onCheckAgain}
          />
        </>
      );
    case "invalid_global_json":
      return (
        <>
          <strong className="error">global.json cannot be used</strong>
          <p data-testid="sdk-project-message">{project.message}</p>
          <p>
            OpenRidr treats global.json as text and never rewrites it. {installLink}
          </p>
          {openGlobalJson}
        </>
      );
    case "not_inspected":
      return (
        <>
          <strong>Not inspected yet</strong>
          <p data-testid="sdk-project-message">
            {project.message} The project SDK is not resolved in Restricted mode, and the .NET 10
            analysis runtime is never used as its substitute.
          </p>
          {pin && <p className="muted">{pin} (read as text).</p>}
        </>
      );
    case "no_workspace":
      return <span className="muted">{project.message}</span>;
  }
}
