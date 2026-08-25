import { escape } from "./layout.mjs";

export const key = (value, strong = false) =>
  `<td class="key${strong ? " strong" : ""}">${escape(value)}</td>`;
export const cell = (value) => `<td>${value ?? ""}</td>`;
export const text = (value) => `<td>${escape(value ?? "")}</td>`;
export const muted = (value) => `<td class="muted">${escape(value ?? "—")}</td>`;

export const STATUS_TONE = {
  BAD_REQUEST: "warning",
  UNPROCESSABLE_ENTITY: "warning",
  CONFLICT: "warning",
  UNAUTHORIZED: "info",
  FORBIDDEN: "danger",
  NOT_FOUND: "neutral",
  TOO_MANY_REQUESTS: "danger",
  INTERNAL_SERVER_ERROR: "danger",
  BAD_GATEWAY: "danger",
  GATEWAY_TIMEOUT: "danger",
  SERVICE_UNAVAILABLE: "danger",
};
export const METHOD_TONE = {
  GET: "info",
  POST: "success",
  PUT: "warning",
  PATCH: "warning",
  DELETE: "danger",
};
