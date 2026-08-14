import { CircleAlert } from "lucide-react";
import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  it.each(["neutral", "info", "warning", "danger", "success"] as const)(
    "renders the %s emphasis tone as an opaque panel",
    (tone) => {
      const { container } = render(
        <StatusBadge tone={tone} icon={<CircleAlert />}>
          Operational state
        </StatusBadge>,
      );

      const badge = container.querySelector("span");
      expect(badge).toHaveClass("rounded", `bg-status-${tone}`, "text-white");
      expect(badge).not.toHaveClass("rounded-full");
      expect(badge?.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    },
  );
});
