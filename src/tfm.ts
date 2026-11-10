/**
 * The TFM a panel shows: the explicit selection when there is one, otherwise the single TFM the
 * evaluated model declares. A project with one TFM has no decision to make, so "Not selected" is
 * never shown for it; a multi-target project keeps the explicit selection and stays empty until
 * the user picks one. The host gate is unchanged — this only decides what the control displays.
 */
export function effectiveTfm(explicit: string | null, declared: string[]): string | null {
  if (explicit) return explicit;
  const tfms = declared.filter((tfm) => tfm.trim().length > 0);
  return tfms.length === 1 ? tfms[0] : null;
}
