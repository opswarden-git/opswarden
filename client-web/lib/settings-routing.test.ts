import { describe, expect, it } from "vitest";
import { settingsView } from "./settings-routing";

describe("settingsView", () => {
  it("keeps the account view as the default", () => {
    expect(settingsView(null)).toBe("general");
    expect(settingsView("unknown")).toBe("general");
  });

  it("recognizes the Team-scoped connector entry point", () => {
    expect(settingsView("connectors")).toBe("connectors");
  });
});
