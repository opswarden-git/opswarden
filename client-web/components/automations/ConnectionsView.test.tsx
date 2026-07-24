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
  it("renders and configures an unknown service entirely from /about.json metadata", () => {
    const service = {
      name: "future-service",
      label: "Future service",
      actions: [],
      reactions: [],
      connection: {
        description: "Catalog-provided connection",
        fields: [
          {
            name: "api_key",
            label: "Future API key",
            description: "Catalog-provided credential",
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

    expect(screen.getByText("Future service")).toBeInTheDocument();
    expect(screen.getByText("Catalog-provided connection")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /connect/i }));
    expect(screen.getByLabelText(/Future API key/)).toHaveAttribute("type", "password");
    expect(screen.getByText("Catalog-provided credential")).toBeInTheDocument();
  });
});
