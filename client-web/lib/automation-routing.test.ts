import { describe, expect, it } from "vitest";
import { automationView, automationWebhookUrl } from "./automation-routing";

describe("automation routing", () => {
  it("accepts known URL-backed views and defaults to rules", () => {
    expect(automationView("connections")).toBe("connections");
    expect(automationView("runs")).toBe("runs");
    expect(automationView("unknown")).toBe("rules");
    expect(automationView(null)).toBe("rules");
  });

  it("falls back to the browser origin for public webhook URLs", () => {
    expect(automationWebhookUrl("/webhooks/github/connection-1", "http://localhost:8081")).toBe(
      "http://localhost:8081/webhooks/github/connection-1",
    );
  });
});
