// Tauri `beforeDevCommand` helper.
//
// Start the client-web dev server ONLY if :4242 is not already serving. The
// OpsWarden dev workflow usually keeps a Next dev server on :4242 alive, and an
// unconditional `next dev` collides with it (EADDRINUSE), which aborts Tauri
// before the window opens. Reuse the running server when present.
import { connect } from "node:net";
import { spawn } from "node:child_process";

const PORT = 4242;

const probe = connect(PORT, "127.0.0.1");

probe.on("connect", () => {
  probe.end();
  console.log(`[desktop] reusing web dev server already on :${PORT}`);
  process.exit(0);
});

probe.on("error", () => {
  console.log(`[desktop] starting client-web dev server on :${PORT}`);
  const child = spawn("npm", ["--prefix", "../client-web", "run", "dev"], {
    stdio: "inherit",
  });
  child.on("exit", (code) => process.exit(code ?? 0));
});
