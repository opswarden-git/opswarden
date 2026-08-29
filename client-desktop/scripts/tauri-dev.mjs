import { spawn } from "node:child_process";
import { developmentOverride } from "./desktop-security.mjs";

const child = spawn(
  "npm",
  ["run", "tauri", "--", "dev", "--config", JSON.stringify(developmentOverride)],
  { stdio: "inherit" },
);

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
