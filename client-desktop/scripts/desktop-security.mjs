import { readFileSync } from "node:fs";

function readJson(relativePath) {
  return JSON.parse(readFileSync(new URL(relativePath, import.meta.url), "utf8"));
}

export const tauriConfig = readJson("../src-tauri/tauri.conf.json");
export const productionCapability = readJson("../src-tauri/capabilities/default.json");

const productionCsp = tauriConfig.app.security.csp;
const developmentCsp = productionCsp.replace(
  "script-src 'self' 'unsafe-inline'",
  "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
);

if (developmentCsp === productionCsp) {
  throw new Error("The desktop production CSP has no canonical script-src directive");
}

const { $schema: _schema, ...developmentCapability } = productionCapability;

/** Dev-only overlay: one different origin and Webpack's eval-based source maps. */
export const developmentOverride = {
  app: {
    security: {
      csp: developmentCsp,
      capabilities: [
        {
          ...developmentCapability,
          identifier: "development-notifications",
          description: "Development Next origin with production-equivalent notification access.",
          remote: { urls: ["http://localhost:4242/*"] },
        },
      ],
    },
  },
};
