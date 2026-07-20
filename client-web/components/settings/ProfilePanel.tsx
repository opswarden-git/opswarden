"use client";

import React, { useState } from "react";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import { ShieldAlert, UserRound } from "lucide-react";
import { useCreateTeam, useTeams } from "@/lib/queries/teams";
import { useAuthStore } from "@/store/auth";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";

/** Station setup (when the user has no team yet) + read-only user identity card. */
export function ProfilePanel() {
  const t = useTranslations("Settings");
  const router = useRouter();
  const params = useParams();
  const currentLocale = params.locale as string;
  const searchParams = useSearchParams();
  const [stationName, setStationName] = useState("");
  const user = useAuthStore((state) => state.user);
  const { data: teams, isLoading: teamsLoading } = useTeams();
  const createTeam = useCreateTeam();
  const needsStationSetup = searchParams.get("setup") === "station" || teams?.length === 0;
  const primaryTeam = teams?.[0];

  const handleCreateStation = (e: React.FormEvent) => {
    e.preventDefault();
    const name = stationName.trim();
    if (!name) return;

    createTeam.mutate(name, {
      onSuccess: () => {
        setStationName("");
        router.replace(`/${currentLocale}/settings`);
      },
    });
  };

  return (
    <>
      {needsStationSetup && (
        <div className="surface border-gold/30 rounded-md p-6 shadow-[inset_0_0_20px_rgba(251,192,45,0.05)]">
          <div className="mb-4 flex items-start gap-3">
            <ShieldAlert className="text-gold mt-0.5 h-5 w-5 shrink-0" />
            <div>
              <h2 className="text-text text-lg font-semibold tracking-tight">{t("setupTitle")}</h2>
              <p className="text-gold/70 mt-1 text-sm">{t("setupDesc")}</p>
            </div>
          </div>
          <form onSubmit={handleCreateStation} className="flex flex-col gap-3 sm:flex-row">
            <input
              type="text"
              value={stationName}
              onChange={(e) => setStationName(e.target.value)}
              placeholder={t("organization")}
              className="ow-input flex h-10 min-w-0 flex-1 rounded-md px-3 py-2 text-sm transition-colors"
            />
            <Button
              type="submit"
              variant="primary"
              size="lg"
              loading={createTeam.isPending}
              disabled={!stationName.trim()}
            >
              {createTeam.isPending ? t("creating") : t("createOrganization")}
            </Button>
          </form>
          {createTeam.isError && (
            <p className="mt-2 text-sm text-red-400">{createTeam.error.message}</p>
          )}
        </div>
      )}

      <div className="surface rounded-md p-6">
        <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
          <UserRound className="text-muted h-5 w-5" />
          {t("user")}
        </h2>
        <div className="mt-4 grid grid-cols-1 gap-6 text-sm sm:grid-cols-2">
          <div>
            <span className="text-muted-2 mb-1 block text-xs font-medium tracking-wider uppercase">
              {t("emailLabel")}
            </span>
            <span className="text-text font-medium">{user?.email ?? t("unknown")}</span>
          </div>
          <div>
            <span className="text-muted-2 mb-1 block text-xs font-medium tracking-wider uppercase">
              {t("userId")}
            </span>
            <span className="text-text font-mono text-xs">{user?.id ?? t("unknown")}</span>
          </div>
          <div>
            <span className="text-muted-2 mb-1 block text-xs font-medium tracking-wider uppercase">
              {t("role")}
            </span>
            <span className="text-text font-medium capitalize">
              {teamsLoading ? t("loading") : (primaryTeam?.role ?? t("noStationYet"))}
            </span>
          </div>
          <div>
            <span className="text-muted-2 mb-1 block text-xs font-medium tracking-wider uppercase">
              {t("organization")}
            </span>
            <span className="text-text font-medium">
              {teamsLoading ? t("loading") : (primaryTeam?.name ?? t("notConfigured"))}
            </span>
          </div>
        </div>
      </div>
    </>
  );
}
