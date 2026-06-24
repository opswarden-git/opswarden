import React, { useEffect, useState } from "react";
import { useTranslations } from "next-intl";
import { X } from "lucide-react";
import { useGifSearch } from "@/lib/queries/gifs";

/**
 * Dense GIPHY search grid shown above the timeline composer. Debounces the
 * query, renders preview stills, and calls `onSelect` with the full-size GIF
 * URL when one is picked. Attribution ("Powered by GIPHY") is always visible.
 */
export function GifSearchPanel({
  onSelect,
  onClose,
  disabled,
}: {
  onSelect: (url: string) => void;
  onClose: () => void;
  disabled?: boolean;
}) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const [term, setTerm] = useState("");
  const [debounced, setDebounced] = useState("");

  useEffect(() => {
    const id = setTimeout(() => setDebounced(term.trim()), 350);
    return () => clearTimeout(id);
  }, [term]);

  const { data, isFetching, error } = useGifSearch(debounced);
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("gifSearchFailed"));

  return (
    <div className="border-border mb-3 rounded-md border p-3">
      <div className="mb-2 flex items-center gap-2">
        <input
          type="text"
          autoFocus
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          placeholder={t("gifSearchPlaceholder")}
          className="ow-input flex h-9 flex-1 rounded-md px-3 py-2 text-sm transition-colors"
        />
        <button
          type="button"
          onClick={onClose}
          aria-label={t("gifClose")}
          className="text-muted hover:text-text transition-colors"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      <div className="min-h-[6rem]">
        {error ? (
          <p className="text-sev-critical py-6 text-center text-xs">{errorText(error.message)}</p>
        ) : debounced.length === 0 ? (
          <p className="text-muted py-6 text-center text-xs">{t("gifSearchHint")}</p>
        ) : isFetching && !data ? (
          <p className="text-muted animate-pulse py-6 text-center text-xs">{t("gifSearching")}</p>
        ) : data && data.length === 0 ? (
          <p className="text-muted py-6 text-center text-xs">{t("gifNoResults")}</p>
        ) : (
          <div className="grid max-h-56 grid-cols-3 gap-2 overflow-y-auto sm:grid-cols-4">
            {data?.map((gif) => (
              <button
                key={gif.id}
                type="button"
                disabled={disabled}
                onClick={() => onSelect(gif.url)}
                title={gif.title}
                className="border-border hover:border-gold/50 aspect-[4/3] overflow-hidden rounded-md border transition-colors disabled:opacity-50"
              >
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={gif.preview_url}
                  alt={gif.title}
                  loading="lazy"
                  className="h-full w-full object-cover"
                />
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="text-muted/60 mt-2 text-right text-[10px] tracking-wide uppercase">
        {t("poweredByGiphy")}
      </div>
    </div>
  );
}
