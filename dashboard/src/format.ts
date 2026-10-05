// Number formatting shared by the dashboard. Angles on the sky are shown
// the way the scope-eval docs show them: arcseconds while small, then
// arcminutes, then degrees.

export function angle(arcsec: number): string {
  const a = Math.abs(arcsec);
  if (!Number.isFinite(a)) return '-';
  if (a < 60) return `${arcsec.toFixed(a < 10 ? 1 : 0)}"`;
  if (a < 3600) return `${(arcsec / 60).toFixed(1)}'`;
  return `${(arcsec / 3600).toFixed(2)} deg`;
}

export function duration(seconds: number): string {
  if (!Number.isFinite(seconds)) return '-';
  const s = Math.max(0, Math.round(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  if (h > 0) return `${h}h ${String(m).padStart(2, '0')}m`;
  if (m > 0) return `${m}m ${String(r).padStart(2, '0')}s`;
  return `${r}s`;
}

/** Elapsed time as m:ss or h:mm:ss, for the transport clock. */
export function clock(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${r}` : `${m}:${r}`;
}

export function percent(fraction: number, digits = 1): string {
  return Number.isFinite(fraction) ? `${(fraction * 100).toFixed(digits)}%` : '-';
}

/** `2008-09-22T00:47:17.209Z` -> `2008-09-22 00:47:17 UTC`. */
export function utc(iso: string): string {
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})/.exec(iso);
  return m ? `${m[1]} ${m[2]} UTC` : iso;
}
