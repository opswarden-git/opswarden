import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AutomationService } from "@/lib/queries/automations";
import { ConnectionsView } from "./ConnectionsView";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string, values?: Record<string, unknown>) =>
    values?.service ? `${key} ${values.service}` : key,
}));

const mutation = {
  error: null,
  isPending: false,
  isSuccess: false,
  mutate: vi.fn(),
  variables: undefined,
};

vi.mock("@/lib/queries/automations", () => ({
  useConfigureTeamConnection: () => mutation,
  useDeleteTeamConnection: () => mutation,
  useRefreshServiceOAuth: () => mutation,
  useStartServiceOAuth: () => mutation,
  useTestTeamConnection: () => mutation,
}));

describe("ConnectionsView", () => {
  it("renders the Generic Webhook connection entirely from /about.json metadata", () => {
    const service = {
      name: "generic",
      label: "Generic Webhook",
      actions: [],
      reactions: [],
      connection: {
        description: "Receive bounded JSON webhooks authenticated with a shared token",
        fields: [
          {
            name: "webhook_signing_secret",
            label: "Shared webhook token",
            description: "Required on first connection; sent in X-OpsWarden-Token",
            input_type: "password",
            required: true,
            default_value: null,
            options: [],
          },
        ],
        oauth: null,
        testable: true,
      },
    } satisfies AutomationService;

    render(<ConnectionsView catalog={[service]} connections={[]} rules={[]} teamId="team-1" />);

    expect(screen.getByText("Generic Webhook")).toBeInTheDocument();
    expect(
      screen.getByText("Receive bounded JSON webhooks authenticated with a shared token"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /connect/i }));
    expect(screen.getByLabelText(/Shared webhook token/)).toHaveAttribute("type", "password");
    expect(
      screen.getByText("Required on first connection; sent in X-OpsWarden-Token"),
    ).toBeInTheDocument();
  });
});
