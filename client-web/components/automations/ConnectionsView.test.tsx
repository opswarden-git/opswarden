import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
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

afterEach(() => vi.clearAllMocks());

describe("ConnectionsView", () => {
  it("uses the official marks for supported providers", () => {
    const catalog = [
      ["github", "GitHub"],
      ["gitlab", "GitLab"],
      ["alertmanager", "Alertmanager"],
    ].map(([name, label]) => ({
      name,
      label,
      actions: [],
      reactions: [],
      connection: {
        description: `${label} connection`,
        fields: [],
        oauth: null,
        testable: false,
      },
    })) satisfies AutomationService[];

    const view = render(
      <ConnectionsView catalog={catalog} connections={[]} rules={[]} teamId="team-1" />,
    );
    const imageSources = Array.from(view.container.querySelectorAll("img"), (image) => image.src);

    expect(imageSources.some((source) => source.includes("github-patched.webp"))).toBe(true);
    expect(imageSources.some((source) => source.includes("gitlab.webp"))).toBe(true);
    expect(imageSources.some((source) => source.includes("alertmanager.svg"))).toBe(true);
    view.unmount();
  });

  it("keeps inactive integrations compact and expands their catalog-driven form", () => {
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

    expect(screen.queryByRole("heading", { name: "activeIntegrations" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "inactiveIntegrations" })).toBeInTheDocument();
    expect(screen.getByText("Generic Webhook")).toBeInTheDocument();
    expect(
      screen.queryByText("Receive bounded JSON webhooks authenticated with a shared token"),
    ).not.toBeInTheDocument();
    const connect = screen.getByRole("button", { name: /connect/i });
    expect(connect).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(connect);
    expect(connect).toHaveAttribute("aria-expanded", "true");
    const secret = screen.getByLabelText(/Shared webhook token/);
    expect(secret).toHaveAttribute("type", "password");
    expect(
      screen.queryByText("Required on first connection; sent in X-OpsWarden-Token"),
    ).not.toBeInTheDocument();
    const connectForm = screen.getByRole("button", { name: "connect" });
    expect(connectForm).toBeDisabled();
    fireEvent.change(secret, { target: { value: "shared-secret" } });
    expect(connectForm).toBeEnabled();
    fireEvent.click(connectForm);
    expect(mutation.mutate).toHaveBeenCalledWith(
      {
        service: "generic",
        payload: { webhook_signing_secret: "shared-secret" },
      },
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });
});
