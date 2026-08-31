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
import { useTeamScope } from "@/components/teams/TeamScope";
import { teamPath } from "@/lib/team-routing";
import { ReleaseDetail } from "./ReleaseDetail";
import { ReleaseStateChip } from "./ReleaseStateChip";
import { actionButtonClassNames, Button } from "@/components/ui/Button";
import { normalizeReleaseView } from "./release-views";
import { formatDateTime } from "@/lib/utils";

function ReleaseDetailSkeleton({ label }: { label: string }) {
  return (
    <div
      className="space-y-6"
      aria-busy="true"
      aria-label={label}
      data-testid="release-detail-skeleton"
    >
      <section className="surface rounded-md p-4 sm:p-6">
        <Skeleton className="h-5 w-40" />
        <div className="mt-4 space-y-4">
          {[0, 1, 2, 3].map((step) => (
            <div key={step} className="flex items-start gap-3">
              <Skeleton className="mt-0.5 h-4 w-4 shrink-0 rounded-full" />
              <div className="min-w-0 flex-1">
                <Skeleton className="h-4 w-36" />
                <Skeleton className="mt-1 h-3 w-24" />
              </div>
              <Skeleton className="h-3 w-32 shrink-0" />
            </div>
          ))}
        </div>
      </section>

      <section className="surface rounded-md p-4 sm:p-6">
        <Skeleton className="h-5 w-32" />
        <div className="mt-4 space-y-3">
          {[0, 1].map((row) => (
            <div key={row} className="flex items-center gap-3">
              <Skeleton className="h-4 min-w-0 flex-1" />
              <Skeleton className="h-5 w-20 shrink-0 rounded-full" />
            </div>
          ))}
        </div>
      </section>
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
  const { activeTeam, role = "observer", capabilities, isLoading: isLoadingTeams } = useTeamScope();
  const { data: release, isLoading, error } = useRelease(releaseId);
  const cancelRelease = useCancelRelease();
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
    isLoadingTeams || isLoading ? "loading" : error || !release || !activeTeam ? "error" : "ready";

  return (
    <PageLayout>
      <PageHeader
        title={
          <span className="flex min-w-0 items-center gap-3">
            <span className="min-w-0 truncate">{release?.title ?? t("deploymentSteps")}</span>
            {release ? <ReleaseStateChip state={release.state} /> : null}
          </span>
        }
        titleAside={
          release ? (
            <time dateTime={release.created_at} className="text-muted text-sm font-normal">
              {t("createdOn", {
                date: formatDateTime(release.created_at, locale),
              })}
            </time>
          ) : null
        }
        actions={
          release && capabilities.canCancelRelease && !terminal ? (
            <Button
              className={actionButtonClassNames()}
              variant="danger"
              loading={cancelRelease.isPending}
              onClick={() => {
                cancelRelease.reset();
                setConfirmCancel(true);
              }}
            >
              {t("cancelRelease")}
            </Button>
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
        {release ? <ReleaseDetail release={release} teamId={teamId} role={role} /> : null}
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
