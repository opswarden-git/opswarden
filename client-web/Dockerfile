# --- client-web/Dockerfile ---
#
# Production web surface for OpsWarden (Next.js), served with `next start` on
# :8081 — the URL-mode target the Tauri desktop build loads. Built from the
# monorepo root context (single root package-lock + npm workspaces). `/api/*` is
# proxied to the server via OPSWARDEN_API_ORIGIN (Compose: http://server:8080).
# Next *bakes* rewrite destinations into the build, so OPSWARDEN_API_ORIGIN is a
# build ARG, not a runtime env. The browser reaches the WebSocket directly on the
# server URL configured by NEXT_PUBLIC_WS_URL (also baked at build time). When
# unset, the client derives `/ws` from the browser origin; Compose supplies its
# explicit localhost URL and the self-hosted ingress routes `/ws` to Rust.

# ---- build ----
FROM node:24-bookworm-slim AS builder
WORKDIR /app
# Manifests first so the `npm ci` layer caches on lockfile changes only.
COPY package.json package-lock.json ./
COPY client-web/package.json ./client-web/package.json
COPY client-desktop/package.json ./client-desktop/package.json
RUN npm ci
# App source + build. Both the client `NEXT_PUBLIC_*` env and the `/api/*` rewrite
# destination are baked into the build, so they are set here as build args.
COPY client-web ./client-web
ARG NEXT_PUBLIC_WS_URL=""
ARG OPSWARDEN_API_ORIGIN=http://localhost:8080
ENV NEXT_PUBLIC_WS_URL=${NEXT_PUBLIC_WS_URL}
ENV OPSWARDEN_API_ORIGIN=${OPSWARDEN_API_ORIGIN}
RUN npm run build --workspace client-web

# ---- runtime ----
FROM node:24-bookworm-slim AS runner
WORKDIR /app/client-web
ENV NODE_ENV=production
# Installed deps + the built app. The /api/* rewrite destination is already
# baked into the build above, so the runtime stage carries no API-origin env.
COPY --from=builder --chown=node:node /app/node_modules /app/node_modules
COPY --from=builder --chown=node:node /app/client-web /app/client-web
USER node
EXPOSE 8081
CMD ["/app/node_modules/.bin/next", "start", "-p", "8081"]
