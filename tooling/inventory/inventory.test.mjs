import assert from "node:assert/strict";
import test from "node:test";

import { formatBytes } from "./extract-server.mjs";
import { METHOD_TONE, statusLabel, statusTone } from "./render-helpers.mjs";

test("formats body limits as operator-readable sizes", () => {
  assert.equal(formatBytes("1024 * 1024"), "1 MiB");
  assert.equal(formatBytes("2 * 1024 * 1024"), "2 MiB");
  assert.equal(formatBytes("1_024 + 512"), "1536 B");
  assert.equal(formatBytes("crate::adapters::webhook::generic::MAX_GENERIC_BODY_BYTES"), "64 KiB");
  assert.equal(formatBytes("unknown_limit()"), "unknown_limit()");
});

test("renders HTTP statuses with their numeric code and consistent tone", () => {
  assert.equal(statusLabel("BAD_REQUEST"), "400 bad request");
  assert.equal(statusLabel("SERVICE_UNAVAILABLE"), "503 service unavailable");
  assert.equal(statusTone("NOT_FOUND"), "warning");
  assert.equal(statusTone("INTERNAL_SERVER_ERROR"), "danger");
});

test("keeps HTTP methods neutral except for destructive requests", () => {
  assert.equal(METHOD_TONE.DELETE, "danger");
  assert.equal(METHOD_TONE.GET, undefined);
  assert.equal(METHOD_TONE.POST, undefined);
});
