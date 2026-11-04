import type { ProjectSdk } from "../host/client";

export function ProjectSdkBody({
  project,
  tfms,
  onOpenGlobalJson,
}: {
  project: ProjectSdk;
  tfms: string[];
  onOpenGlobalJson: () => void;
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

  switch (project.state) {
    case "resolved":
      return (
        <>
          <strong className="success">SDK {project.resolvedVersion}</strong>{" "}
          <span className="tag">resolved after trust · not a build</span>
          <p>
            {pin}
            {project.allowPrerelease ? " · prerelease allowed" : ""}.
          </p>
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
          <p data-testid="sdk-project-message">{project.message}</p>
          <p>
            {pin ?? "global.json does not declare an SDK version."} Install that SDK or change
            global.json yourself; this build does neither and never substitutes the .NET 10
            analysis runtime.
          </p>
          <p>{installLink}</p>
          <button type="button" data-testid="sdk-open-global-json" onClick={onOpenGlobalJson}>
            Open global.json
          </button>
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
          <button type="button" data-testid="sdk-open-global-json" onClick={onOpenGlobalJson}>
            Open global.json
          </button>
        </>
      );
    case "not_pinned":
      return (
        <>
          <strong>Not pinned</strong>
          <p data-testid="sdk-project-message">{project.message}</p>
          <p>
            {declared
              ? `The declared TargetFramework (${declared}) decides which SDK MSBuild needs. `
              : "The project TargetFramework decides which SDK MSBuild needs. "}
            Add sdk.version to global.json to pin it explicitly.
          </p>
          <button type="button" data-testid="sdk-open-global-json" onClick={onOpenGlobalJson}>
            Open global.json
          </button>
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
    case "no_global_json":
      return (
        <>
          <strong>No global.json</strong>
          <p data-testid="sdk-project-message">{project.message}</p>
          <p>
            {declared
              ? `Without a pin, the declared TargetFramework (${declared}) decides which SDK MSBuild needs. `
              : "Without a pin, the project TargetFramework decides which SDK MSBuild needs. "}
            Add global.json to resolve one explicitly.
          </p>
        </>
      );
    case "no_workspace":
      return <span className="muted">{project.message}</span>;
  }
}
