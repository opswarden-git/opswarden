export const RELEASE_STATES = [
  "created",
  "in_progress",
  "blocked",
  "completed",
  "cancelled",
] as const;

export type ReleaseState = (typeof RELEASE_STATES)[number];
