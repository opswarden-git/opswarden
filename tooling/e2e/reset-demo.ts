import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import {
  DEMO_CONTRACTOR_EMAIL,
  DEMO_MANAGER_EMAIL,
  DEMO_OBSERVER_EMAIL,
  DEMO_PASSWORD,
  DEMO_RESPONDER_EMAIL,
  DEMO_TEAM_ID,
  DEMO_TEAM_NAME,
} from "./demo-env";

const repositoryRoot = resolve(__dirname, "../..");

export default function resetDemo() {
  execFileSync(
    "python3",
    [resolve(repositoryRoot, "tooling/demo.py"), "seed", "--target", "local", "--data-only"],
    {
      cwd: repositoryRoot,
      env: {
        ...process.env,
        DEMO_LOCAL_API_ORIGIN: "http://localhost:8080",
        DEMO_LOCAL_PASSWORD: DEMO_PASSWORD,
        DEMO_LOCAL_MANAGER_EMAIL: DEMO_MANAGER_EMAIL,
        DEMO_LOCAL_RESPONDER_EMAIL: DEMO_RESPONDER_EMAIL,
        DEMO_LOCAL_OBSERVER_EMAIL: DEMO_OBSERVER_EMAIL,
        DEMO_LOCAL_CONTRACTOR_EMAIL: DEMO_CONTRACTOR_EMAIL,
        DEMO_LOCAL_TEAM_ID: DEMO_TEAM_ID,
        DEMO_LOCAL_TEAM_NAME: DEMO_TEAM_NAME,
      },
      stdio: "inherit",
    },
  );
}
