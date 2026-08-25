"use client";

import React from "react";
import { useLocale, useTranslations } from "next-intl";
import { useSearchParams } from "next/navigation";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { Alert } from "@/components/ui/Alert";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Skeleton } from "@/components/ui/Skeleton";
import { Link, useRouter } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { useCancelRelease, useRelease } from "@/lib/queries/releases";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { ReleaseDetail } from "./ReleaseDetail";
import { normalizeReleaseView } from "./release-views";

function ReleaseDetailSkeleton({ label }: { label: string }) {
  return (
    <div
      className="grid grid-cols-1 gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]"
      aria-busy="true"
      aria-label={label}
      data-testid="release-detail-skeleton"
    >
      <section className="surface rounded-md p-5 sm:p-6">
        <Skeleton className="h-5 w-40" />
        <Skeleton className="mt-2 h-4 w-64 max-w-full" />

        <div className="mt-7 space-y-6">
          {[0, 1, 2, 3].map((step) => (
            <div key={step} className="flex gap-4">
              <div className="relative flex w-8 shrink-0 justify-center">
                {step < 3 ? (
                  <span
                    className="bg-border absolute top-8 -bottom-6 left-1/2 w-px -translate-x-1/2"
                    aria-hidden="true"
                  />
                ) : null}
                <Skeleton className="relative z-10 h-8 w-8 rounded-full" />
              </div>
              <div className="min-w-0 flex-1 pt-1">
                <Skeleton className="h-4 w-36" />
                <Skeleton className="mt-2 h-3 w-52 max-w-full" />
              </div>
            </div>
          ))}
        </div>
      </section>

      <aside className="surface h-fit overflow-hidden rounded-md">
        <div className="border-border border-b px-4 py-3">
          <Skeleton className="h-4 w-24" />
        </div>
        <div className="divide-border divide-y">
          <div className="space-y-4 p-4">
            <div className="flex items-center justify-between gap-4">
              <Skeleton className="h-3 w-14" />
              <Skeleton className="h-6 w-20 rounded-full" />
            </div>
            <div className="flex items-center justify-between gap-4">
              <Skeleton className="h-3 w-16" />
              <Skeleton className="h-3 w-10" />
            </div>
          </div>
          <div className="space-y-3 p-4">
            <Skeleton className="h-3 w-16" />
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-9 w-full rounded-md" />
          </div>
          <div className="space-y-3 p-4">
            <Skeleton className="h-3 w-24" />
            <Skeleton className="h-9 w-full rounded-md" />
          </div>
        </div>
      </aside>
    </div>
  );
}

export function ReleaseDetailPage({ teamId, releaseId }: { teamId: string; releaseId: string }) {
  const t = useTranslations("Releases");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const router = useRouter();
  const searchParams = useSearchParams();
  const [confirmCancel, setConfirmCancel] = React.useState(false);
  const { data: teams, isLoading: isLoadingTeams } = useTeams();
  const { data: release, isLoading, error } = useRelease(releaseId);
  const cancelRelease = useCancelRelease();

  const team = teams?.find((candidate) => candidate.team_id === teamId);
  const capabilities = deriveCapabilities(team?.role ?? "observer");
  const view = normalizeReleaseView(searchParams.get("view"));
  const listBase = teamPath(teamId, "releases");
  const listHref = view === "active" ? listBase : `${listBase}?view=${view}`;
  const terminal = release?.state === "completed" || release?.state === "cancelled";
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  React.useEffect(() => {
    if (!release || release.team_id === teamId) return;
    const target = teamPath(release.team_id, "releases", release.release_id);
    router.replace(view === "active" ? target : `${target}?view=${view}`);
  }, [release, router, teamId, view]);

  const state: PageContentState =
    isLoadingTeams || isLoading ? "loading" : error || !release || !team ? "error" : "ready";

  return (
    <PageLayout>
      <PageHeader
        title={release?.title ?? t("releaseDetail")}
        metadata={
          release ? (
            <time dateTime={release.created_at}>
              {t("createdOn", {
                date: new Intl.DateTimeFormat(locale, {
                  dateStyle: "medium",
                  timeStyle: "short",
                }).format(new Date(release.created_at)),
              })}
            </time>
          ) : null
        }
      />

      <PageContent
        state={state}
        loadingFallback={<ReleaseDetailSkeleton label={t("loading")} />}
        errorFallback={
          <Alert tone="danger" title={t("failedToLoadDetail")}>
            <Link href={listHref} className="underline">
              {t("backToReleases")}
            </Link>
          </Alert>
        }
      >
        {release ? (
          <ReleaseDetail
            release={release}
            teamId={teamId}
            role={team?.role ?? "observer"}
            cancelPending={cancelRelease.isPending}
            onCancel={
              capabilities.canCancelRelease && !terminal
                ? () => {
                    cancelRelease.reset();
                    setConfirmCancel(true);
                  }
                : undefined
            }
          />
        ) : null}
      </PageContent>

      {release ? (
        <ConfirmDialog
          open={confirmCancel}
          title={t("cancelRelease")}
          description={t("cancelConfirm", { title: release.title })}
          confirmLabel={t("cancelRelease")}
          cancelLabel={t("keep")}
          intent="destructive"
          pendingLabel={t("processing")}
          pending={cancelRelease.isPending}
          error={cancelRelease.error ? errorText(cancelRelease.error.message) : null}
          onConfirm={() =>
            cancelRelease.mutate(
              { releaseId: release.release_id, teamId },
              { onSuccess: () => setConfirmCancel(false) },
            )
          }
          onClose={() => setConfirmCancel(false)}
        />
      ) : null}
    </PageLayout>
  );
}
