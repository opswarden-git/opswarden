import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { developmentOverride, productionCapability, tauriConfig } from "./desktop-security.mjs";

const notificationPermissions = [
  "notification:allow-is-permission-granted",
  "notification:allow-request-permission",
  "notification:allow-notify",
];

test("production exposes no global API and grants only required notification commands", () => {
  assert.equal(tauriConfig.build.frontendDist, "http://localhost:8081");
  assert.equal(tauriConfig.app.withGlobalTauri, false);
  assert.deepEqual(tauriConfig.app.security.capabilities, ["default"]);
  assert.equal(productionCapability.local, false);
  assert.deepEqual(productionCapability.remote.urls, ["http://localhost:8081/*"]);
  assert.deepEqual(productionCapability.permissions, notificationPermissions);
  assert.ok(!JSON.stringify(productionCapability).includes("core:default"));
  assert.ok(!JSON.stringify(productionCapability).includes("notification:default"));
});

test("development access is generated from production permissions without leaking into bundles", () => {
  const [capability] = developmentOverride.app.security.capabilities;
  assert.deepEqual(capability.remote.urls, ["http://localhost:4242/*"]);
  assert.deepEqual(capability.permissions, notificationPermissions);
  assert.ok(developmentOverride.app.security.csp.includes("'unsafe-eval'"));
  assert.ok(!tauriConfig.app.security.csp.includes("'unsafe-eval'"));
});

test("production CSP closes executable, embedding and navigation defaults", () => {
  const csp = tauriConfig.app.security.csp;
  for (const directive of [
    "default-src 'self'",
    "base-uri 'self'",
    "object-src 'none'",
    "frame-ancestors 'none'",
    "form-action 'self'",
    "script-src 'self' 'unsafe-inline'",
    "connect-src 'self' ipc: http://ipc.localhost ws: wss:",
  ]) {
    assert.ok(csp.includes(directive), `missing CSP directive: ${directive}`);
  }
});

test("the native notification adapter does not depend on the removed global API", () => {
  const source = readFileSync(
    new URL("../../client-web/lib/desktopNotify.ts", import.meta.url),
    "utf8",
  );
  assert.match(source, /@tauri-apps\/plugin-notification/);
  assert.doesNotMatch(source, /window\.__TAURI__\s*[.[]/);
});
