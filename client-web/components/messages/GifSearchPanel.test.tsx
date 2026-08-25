import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GifSearchPanel } from "./GifSearchPanel";

vi.mock("next-intl", () => ({
  useTranslations: () => {
    const translate = (key: string) => key;
    translate.has = () => true;
    return translate;
  },
}));
vi.mock("@/lib/queries/gifs", () => ({
  useGifSearch: (query: string) => ({
    data: undefined,
    error: null,
    isFetching: query.length > 0,
  }),
}));

afterEach(cleanup);

describe("GifSearchPanel loading morphology", () => {
  it("reserves the final responsive media grid while results load", async () => {
    render(<GifSearchPanel onSelect={vi.fn()} onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText("gifSearchPlaceholder"), {
      target: { value: "incident" },
    });

    const skeleton = await screen.findByLabelText("gifSearching", {}, { timeout: 1000 });
    await waitFor(() => expect(skeleton.querySelectorAll(".animate-pulse")).toHaveLength(8));
    expect(skeleton).toHaveClass("grid-cols-3", "sm:grid-cols-4");
  });
});
