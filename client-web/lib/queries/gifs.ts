import { useQuery } from "@tanstack/react-query";
import { apiFetch } from "../api";

export interface GifResult {
  id: string;
  title: string;
  url: string;
  preview_url: string;
  width: number;
  height: number;
}

/** Sentinel prefix for a timeline entry that holds a GIF instead of text. The
 * GIF is stored as `giphy:<url>` so no DB migration is needed; it rides in the
 * existing `content` column. */
const GIPHY_PREFIX = "giphy:";

/**
 * If `content` is a GIPHY timeline entry (`giphy:<url>`) whose URL is an https
 * giphy.com host, return the URL; otherwise null. The host allowlist stops an
 * arbitrary attacker-controlled image URL from being smuggled through the
 * sentinel and rendered as an `<img>`.
 */
export function giphyEntryUrl(content: string): string | null {
  if (!content.startsWith(GIPHY_PREFIX)) return null;
  try {
    const url = new URL(content.slice(GIPHY_PREFIX.length));
    if (url.protocol !== "https:") return null;
    const host = url.hostname;
    if (host === "giphy.com" || host.endsWith(".giphy.com")) return url.toString();
    return null;
  } catch {
    return null;
  }
}

/**
 * Search GIPHY through the server-side proxy (the API key never reaches the
 * browser). Disabled until `query` is non-empty so we never fire a blank
 * search; results are cached briefly to keep typing responsive.
 */
export function useGifSearch(query: string) {
  const q = query.trim();
  return useQuery<GifResult[]>({
    queryKey: ["gif-search", q],
    queryFn: async () => {
      const res = await apiFetch(`/api/giphy/search?q=${encodeURIComponent(q)}&limit=18`);
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "gif_search_failed");
      }
      return (await res.json()) as GifResult[];
    },
    enabled: q.length > 0,
    staleTime: 60_000,
  });
}
