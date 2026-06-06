// Small display helpers shared across the surfaces. The engine speaks in
// snake_case decision/status tokens; the UI humanizes them for reading.

/** Turn an engine token like `reverse_charge_applies` into `Reverse charge applies`. */
export function humanize(token: string): string {
  const spaced = token.replace(/_/g, " ").trim();
  if (!spaced) return spaced;
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

/** Render a 0..1 accuracy as a percentage, or an em-free dash when absent. */
export function formatAccuracy(value: number | null | undefined): string {
  if (value === null || value === undefined) return "n/a";
  return `${(value * 100).toFixed(1)}%`;
}

/** Best-effort local date/time from an engine timestamp string. */
export function formatTimestamp(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;
  return parsed.toLocaleString();
}
