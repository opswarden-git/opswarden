"use client";

import React from "react";
import { Ban, Rocket } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { useSearchParams } from "next/navigation";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { ActionMenu } from "@/components/ui/ActionMenu";
import { Alert } from "@/components/ui/Alert";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Link, useRouter } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { useCancelRelease, useRelease } from "@/lib/queries/releases";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { ReleaseDetail } from "./ReleaseDetail";
import { ReleaseStateChip } from "./ReleaseStateChip";
import { normalizeReleaseView } from "./release-views";

function ReleaseBreadcrumb({
  listHref,
  releaseTitle,
  teamHref,
  teamName,
}: {
  listHref: string;
  releaseTitle: string;
  teamHref: string;
  teamName: string;
}) {
  const t = useTranslations("Releases");

  return (
    <nav aria-label={t("breadcrumbLabel")} className="text-muted min-w-0 text-sm">
      <ol className="flex min-w-0 items-center gap-2">
        <li className="min-w-0 truncate">
          <Link href={teamHref} className="hover:text-text transition-colors">
            {teamName}
          </Link>
        </li>
        <li aria-hidden="true">/</li>
        <li>
          <Link href={listHref} className="hover:text-text transition-colors">
            {t("title")}
          </Link>
        </li>
        <li aria-hidden="true">/</li>
        <li className="text-text min-w-0 truncate font-medium" aria-current="page">
          {releaseTitle}
        </li>
      </ol>
    </nav>
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
  const completedSteps = release?.steps.filter((step) => step.validated).length ?? 0;
  const totalSteps = release?.steps.length ?? 0;
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  React.useEffect(() => {
    if (!release || release.team_id === teamId) return;
    const target = teamPath(release.team_id, "releases", release.release_id);
    router.replace(view === "active" ? target : `${target}?view=${view}`);
  }, [release, router, teamId, view]);

  const state: PageContentState =
    isLoadingTeams || isLoading ? "loading" : error || !release || !team ? "error" : "ready";

  return (
    <PageLayout width="workspace">
      {release && team ? (
        <ReleaseBreadcrumb
          listHref={listHref}
          releaseTitle={release.title}
          teamHref={teamPath(teamId, "overview")}
          teamName={team.name}
        />
      ) : null}

      <PageHeader
        title={release?.title ?? t("releaseDetail")}
        description={t("detailDescription")}
        metadata={
          release ? (
            <div className="flex flex-wrap items-center gap-3">
              <ReleaseStateChip state={release.state} />
              <span>{t("progressCount", { completed: completedSteps, total: totalSteps })}</span>
              <time dateTime={release.created_at}>
                {t("createdOn", {
                  date: new Intl.DateTimeFormat(locale, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }).format(new Date(release.created_at)),
                })}
              </time>
            </div>
          ) : null
        }
        actions={
          release && capabilities.canCancelRelease && !terminal ? (
            <ActionMenu
              label={t("moreActions")}
              items={[
                {
                  id: "cancel",
                  label: t("cancelRelease"),
                  icon: Ban,
                  tone: "danger",
                  onSelect: () => {
                    cancelRelease.reset();
                    setConfirmCancel(true);
                  },
                },
              ]}
            />
          ) : null
        }
      />

      <PageContent
        state={state}
        loadingFallback={
          <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]">
            <div className="surface h-[32rem] animate-pulse rounded-md" />
            <div className="surface h-64 animate-pulse rounded-md" />
          </div>
        }
        errorFallback={
          <Alert tone="danger" title={t("failedToLoadDetail")}>
            <Link href={listHref} className="underline">
              {t("backToReleases")}
            </Link>
          </Alert>
        }
      >
        {release ? (
          <ReleaseDetail release={release} teamId={teamId} role={team?.role ?? "observer"} />
        ) : null}
      </PageContent>

      {release ? (
        <ConfirmDialog
          open={confirmCancel}
          title={t("cancelRelease")}
          description={t("cancelConfirm", { title: release.title })}
          confirmLabel={t("cancelRelease")}
          cancelLabel={t("keep")}
          pendingLabel={t("processing")}
          danger
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
