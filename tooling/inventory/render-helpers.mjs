import { escape } from "./layout.mjs";

export const key = (value, strong = false) =>
  `<td class="key${strong ? " strong" : ""}">${escape(value)}</td>`;
export const cell = (value) => `<td>${value ?? ""}</td>`;
export const text = (value) => `<td>${escape(value ?? "")}</td>`;
export const muted = (value) => `<td class="muted">${escape(value ?? "—")}</td>`;

/**
 * Statuts HTTP : le numéro manquait, et les tons étaient distribués au jugé
 * (bad gateway en danger, bad request en warning, not found en neutre). La
 * charte ne définit que cinq tons opérationnels, on s'y tient : une 4xx est
 * une attention sans échec, une 5xx est un échec.
 */
export const STATUS_CODE = {
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  CONFLICT: 409,
  UNPROCESSABLE_ENTITY: 422,
  TOO_MANY_REQUESTS: 429,
  INTERNAL_SERVER_ERROR: 500,
  BAD_GATEWAY: 502,
  SERVICE_UNAVAILABLE: 503,
  GATEWAY_TIMEOUT: 504,
};

export const statusTone = (status) => ((STATUS_CODE[status] ?? 0) >= 500 ? "danger" : "warning");

/** `400 bad request`, jamais l'un sans l'autre. */
export const statusLabel = (status) =>
  `${STATUS_CODE[status] ?? "?"} ${status.replace(/_/g, " ").toLowerCase()}`;

/**
 * Un verbe HTTP n'est pas un état opérationnel : le colorer avec les tons de
 * la charte faisait lire « DELETE » comme une escalade. Neutre partout, sauf
 * DELETE, la seule action que la charte demande de signaler comme destructrice.
 */
export const METHOD_TONE = { DELETE: "danger" };
