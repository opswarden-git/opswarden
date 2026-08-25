import React, { useEffect, useState } from "react";
import { useTranslations } from "next-intl";
import { X } from "lucide-react";
import { useGifSearch } from "@/lib/queries/gifs";
import { IconButton } from "@/components/ui/Button";
import { MediaButton } from "@/components/ui/MediaButton";
import { Skeleton } from "@/components/ui/Skeleton";

/** Shared GIPHY picker for every conversation composer. */
export function GifSearchPanel({
  onSelect,
  onClose,
  disabled,
}: {
  onSelect: (url: string) => void;
  onClose: () => void;
  disabled?: boolean;
}) {
  const t = useTranslations("Common");
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
        <label className="min-w-0 flex-1">
          <span className="sr-only">{t("gifButton")}</span>
          <input
            type="text"
            autoFocus
            value={term}
            onChange={(event) => setTerm(event.target.value)}
            placeholder={t("gifSearchPlaceholder")}
            className="ow-input focus-visible:border-border flex h-9 w-full rounded-md px-3 py-2 text-sm transition-colors focus-visible:shadow-none"
          />
        </label>
        <IconButton label={t("gifClose")} size="sm" variant="ghost" onClick={onClose}>
          <X className="h-4 w-4" aria-hidden="true" />
        </IconButton>
      </div>

      <div className="min-h-[6rem]">
        {error ? (
          <p className="text-sev-critical py-6 text-center text-xs" role="alert">
            {errorText(error.message)}
          </p>
        ) : debounced.length === 0 ? (
          <p className="text-muted py-6 text-center text-xs">{t("gifSearchHint")}</p>
        ) : isFetching && !data ? (
          <div
            className="grid max-h-56 grid-cols-3 gap-2 overflow-hidden sm:grid-cols-4"
            aria-busy="true"
            aria-label={t("gifSearching")}
          >
            {Array.from({ length: 8 }, (_, index) => (
              <Skeleton key={index} className="aspect-video w-full rounded-md" />
            ))}
          </div>
        ) : data && data.length === 0 ? (
          <p className="text-muted py-6 text-center text-xs">{t("gifNoResults")}</p>
        ) : (
          <div className="grid max-h-56 grid-cols-3 gap-2 overflow-y-auto sm:grid-cols-4">
            {data?.map((gif) => (
              <MediaButton
                key={gif.id}
                label={gif.title}
                disabled={disabled}
                onClick={() => onSelect(gif.url)}
                title={gif.title}
              >
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={gif.preview_url}
                  alt={gif.title}
                  loading="lazy"
                  className="h-full w-full object-cover"
                />
              </MediaButton>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
