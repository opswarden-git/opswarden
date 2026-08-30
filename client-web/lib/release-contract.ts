export const RELEASE_STATES = [
  "created",
  "in_progress",
  "blocked",
  "completed",
  "cancelled",
] as const;

export type ReleaseState = (typeof RELEASE_STATES)[number];

export function isReleaseState(value: string): value is ReleaseState {
  return (RELEASE_STATES as readonly string[]).includes(value);
}
