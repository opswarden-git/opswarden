"use client";

import { ImagePlus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslations } from "next-intl";
import { apiFetch } from "@/lib/api";
import { type Team, useDeleteTeamImage, useUpdateTeamImage } from "@/lib/queries/teams";
import { IconButton } from "@/components/ui/Button";

const MAX_TEAM_IMAGE_BYTES = 1024 * 1024;
const ACCEPTED_TYPES = new Set(["image/jpeg", "image/png", "image/webp"]);

export function TeamImageEditor({
  canEdit,
  fallback,
  team,
}: {
  canEdit: boolean;
  fallback: string;
  team: Team;
}) {
  const t = useTranslations("Teams");
  const input = useRef<HTMLInputElement>(null);
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [clientError, setClientError] = useState(false);
  const update = useUpdateTeamImage(team.team_id);
  const remove = useDeleteTeamImage(team.team_id);

  useEffect(() => {
    let objectUrl: string | null = null;
    let cancelled = false;
    if (!team.image_updated_at) return;
    void apiFetch(`/api/teams/${team.team_id}/image?v=${encodeURIComponent(team.image_updated_at)}`)
      .then(async (response) => {
        if (!response.ok) throw new Error("team_image_load_failed");
        objectUrl = URL.createObjectURL(await response.blob());
        if (!cancelled) setImageUrl(objectUrl);
      })
      .catch(() => {
        if (!cancelled) setImageUrl(null);
      });
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [team.image_updated_at, team.team_id]);

  const visibleImageUrl = team.image_updated_at ? imageUrl : null;
  const content = visibleImageUrl ? (
    // The resource is authorization-gated and loaded through apiFetch.
    // eslint-disable-next-line @next/next/no-img-element
    <img src={visibleImageUrl} alt="" className="h-full w-full rounded-full object-cover" />
  ) : (
    fallback
  );

  if (!canEdit) return content;

  const hasError = clientError || update.isError || remove.isError;
  return (
    <span className="group relative flex h-full w-full items-center justify-center">
      <button
        type="button"
        className="focus-visible:ring-gold/50 relative flex h-full w-full items-center justify-center overflow-hidden rounded-full focus-visible:ring-2 focus-visible:outline-none"
        aria-label={t("changeTeamImage")}
        onClick={() => input.current?.click()}
      >
        {content}
        <span className="bg-bg/70 absolute inset-0 hidden items-center justify-center group-focus-within:flex group-hover:flex">
          <ImagePlus className="h-5 w-5" aria-hidden="true" />
        </span>
      </button>
      <input
        ref={input}
        type="file"
        aria-label={t("changeTeamImage")}
        accept="image/jpeg,image/png,image/webp"
        className="sr-only"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (!file) return;
          setClientError(!ACCEPTED_TYPES.has(file.type) || file.size > MAX_TEAM_IMAGE_BYTES);
          if (!ACCEPTED_TYPES.has(file.type) || file.size > MAX_TEAM_IMAGE_BYTES) return;
          update.mutate(file);
        }}
      />
      {team.image_updated_at ? (
        <IconButton
          label={t("removeTeamImage")}
          size="xs"
          tone="danger"
          className="absolute -right-2 -bottom-2 rounded-full"
          loading={remove.isPending}
          onClick={() => remove.mutate()}
        >
          <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
        </IconButton>
      ) : null}
      {hasError ? (
        <span className="sr-only" role="alert">
          {t("actionFailed")}
        </span>
      ) : null}
    </span>
  );
}
