import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

const repositoryRoot = resolve(__dirname, "../..");

export default function resetDemo() {
  execFileSync(
    "python3",
    [resolve(repositoryRoot, "tooling/demo.py"), "seed", "--target", "local", "--data-only"],
    {
      cwd: repositoryRoot,
      stdio: "inherit",
    },
  );
}
