import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

const repositoryRoot = resolve(__dirname, "../..");

export default function resetDemo() {
  execFileSync(resolve(repositoryRoot, "tooling/seed_demo.sh"), [], {
    cwd: repositoryRoot,
    stdio: "inherit",
  });
}
