import { fileIconUrl } from "./iconAssets";

/**
 * The NuGet drawing: the Dependencies node, the packages under it and the frameworks it brings.
 * The theme has no framework icon of its own, so a framework row draws the same one.
 *
 * The class name is the row's hook (`.dependency-glyph`, `.runtime-glyph`, `.package-glyph`); the
 * drawing is the theme's, in the theme's colours.
 */
export function NugetGlyph({ className }: { className?: string }) {
  return (
    <img
      className={className}
      data-icon="nuget"
      src={fileIconUrl("nuget")}
      width={16}
      height={16}
      alt=""
      aria-hidden="true"
    />
  );
}
