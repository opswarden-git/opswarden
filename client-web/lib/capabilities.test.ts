import { describe, expect, it } from "vitest";
import contract from "../../contracts/role-capabilities.json";
import { deriveCapabilities, type TeamRole } from "./capabilities";

describe("deriveCapabilities", () => {
  it.each(["observer", "responder", "manager"] satisfies TeamRole[])(
    "matches the shared contract for %s",
    (role) => {
      expect(deriveCapabilities(role)).toEqual(contract[role]);
    },
  );
});
