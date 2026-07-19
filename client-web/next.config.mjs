import createNextIntlPlugin from "next-intl/plugin";
import path from "node:path";
import { fileURLToPath } from "node:url";

const withNextIntl = createNextIntlPlugin("./i18n/request.ts");
const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

// Origin the Next server proxies `/api/*` to. Next *bakes* rewrite destinations
// into the build, so this is read at build time: the Compose image is built with
// `OPSWARDEN_API_ORIGIN=http://server:8080` (internal network), while `next dev`
// re-evaluates the config per run and falls back to the host server on :8080.
// The WebSocket is a separate, browser-direct concern (NEXT_PUBLIC_WS_URL).
const apiOrigin = process.env.OPSWARDEN_API_ORIGIN || "http://localhost:8080";

/** @type {import('next').NextConfig} */
const nextConfig = {
  turbopack: {
    root: workspaceRoot,
  },
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: `${apiOrigin}/api/:path*`,
      },
      {
        source: "/about.json",
        destination: `${apiOrigin}/about.json`,
      },
    ];
  },
};

export default withNextIntl(nextConfig);
