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
FROM node:24-bookworm-slim@sha256:6f7b03f7c2c8e2e784dcf9295400527b9b1270fd37b7e9a7285cf83b6951452d AS builder
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
FROM node:24-bookworm-slim@sha256:6f7b03f7c2c8e2e784dcf9295400527b9b1270fd37b7e9a7285cf83b6951452d AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV HOSTNAME=0.0.0.0
ENV PORT=8081
# Next's traced standalone server contains only the production files required
# at runtime. Build tooling and the monorepo-wide node_modules never cross this
# stage boundary.
COPY --from=builder --chown=node:node /app/client-web/.next/standalone ./
COPY --from=builder --chown=node:node /app/client-web/.next/static ./client-web/.next/static
COPY --from=builder --chown=node:node /app/client-web/public ./client-web/public
USER node
EXPOSE 8081
CMD ["node", "client-web/server.js"]
