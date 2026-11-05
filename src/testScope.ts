/**
 * The TFM the Tests panel operates on.
 *
 * The current evaluation wins: a discovery made on `net8.0` must not keep the selector on
 * `net8.0` after the model moved to `net9.0`, because every action in the panel runs against the
 * evaluated context, not against the earlier discovery. With no explicit selection, only the
 * selected test project's own single TFM is named — a solution-wide "the one TFM everyone
 * declares" is not evidence that this project declares it.
 */
export function testPanelFramework(args: {
  /** `evaluation.context.targetFramework`: the explicit selection, or what the model evaluated. */
  contextFramework: string | null;
  /** `tfms` of the selected test project (empty when it was not evaluated). */
  projectTfms: string[];
}): string | null {
  if (args.contextFramework) return args.contextFramework;
  const declared = args.projectTfms.filter((tfm) => tfm.trim().length > 0);
  return declared.length === 1 ? declared[0] : null;
}
