import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const root = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": root,
    },
  },
  test: {
    environment: "jsdom",
    // Keep GitHub's output deterministic. CI writes a concise project-owned
    // summary instead of Vitest's automatic "passes / total" wording.
    reporters: ["default"],
    setupFiles: ["./vitest.setup.ts"],
    coverage: {
      include: [
        "components/**/*.{ts,tsx}",
        "i18n/**/*.{ts,tsx}",
        "lib/**/*.{ts,tsx}",
        "store/**/*.{ts,tsx}",
      ],
      exclude: ["**/*.test.*", "**/types.ts"],
      reporter: ["text", "html", "json-summary", "lcov"],
      thresholds: {
        lines: 70,
        statements: 65,
        functions: 65,
        branches: 55,
      },
    },
  },
});
