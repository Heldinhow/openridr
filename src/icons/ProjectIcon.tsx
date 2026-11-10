import { fileIconUrl } from "./iconAssets";

/**
 * A project row: the theme's `.csproj` drawing. The theme uses one Visual Studio icon for both the
 * solution file and the project file, so the solution root and its projects share it; the project
 * kind stays readable as text in the row's second layer.
 */
export function ProjectIcon() {
  return (
    <img
      className="project-icon"
      data-icon="visualstudio"
      src={fileIconUrl("visualstudio")}
      width={16}
      height={16}
      alt=""
      aria-hidden="true"
    />
  );
}
