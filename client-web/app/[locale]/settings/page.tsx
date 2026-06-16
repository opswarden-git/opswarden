"use client";

import React, { useState } from "react";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import {
  AlertTriangle,
  Languages,
  LogOut,
  PencilLine,
  ShieldAlert,
  Sliders,
  Trash2,
  UserRound,
  Workflow,
} from "lucide-react";
import Image from "next/image";
import { useRouter as useIntlRouter, usePathname } from "../../../i18n/routing";
import { apiFetch } from "@/lib/api";
import { useCreateTeam, useTeams } from "@/lib/queries/teams";
import { useAuthStore } from "@/store/auth";
import { useTranslations } from "next-intl";

const AVAILABLE_INTEGRATIONS = [
  {
    id: "github",
    name: "GitHub",
    desc: "Link actions & deployment flows",
    icon: "/assets/github-patched.webp",
  },
  {
    id: "gitlab",
    name: "GitLab",
    desc: "Sync pipelines and issue boards",
    icon: "/assets/gitlab.webp",
  },
  {
    id: "k8s",
    name: "Kubernetes",
    desc: "Deploy container metrics monitor",
    icon: "/assets/kubernetes.webp",
  },
  {
    id: "sentry",
    name: "Sentry",
    desc: "Track application exceptions & crashes",
    icon: "/assets/sentry.webp",
  },
  {
    id: "datadog",
    name: "Datadog",
    desc: "Sync system APM telemetry data",
    icon: "/assets/datadog.webp",
  },
  {
    id: "pagerduty",
    name: "PagerDuty",
    desc: "Sync incident & rotation escalations",
    icon: "/assets/pagerduty.webp",
  },
];

export default function SettingsPage() {
  const router = useRouter();
  const t = useTranslations("Settings");
  const tErr = useTranslations("errors");
  const searchParams = useSearchParams();
  const [activeTab, setActiveTab] = useState<"profile" | "integrations">("profile");
  const [connectedList, setConnectedList] = useState<string[]>(["github", "k8s"]);
  const [stationName, setStationName] = useState("");
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState("");
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [deletePending, setDeletePending] = useState(false);

  const intlRouter = useIntlRouter();
  const pathname = usePathname();
  const params = useParams();
  const currentLocale = params.locale as string;
  const user = useAuthStore((state) => state.user);
  const logoutLocal = useAuthStore((state) => state.logout);
  const { data: teams, isLoading: teamsLoading } = useTeams();
  const createTeam = useCreateTeam();
  const needsStationSetup = searchParams.get("setup") === "station" || teams?.length === 0;
  const primaryTeam = teams?.[0];

  const switchLocale = (newLocale: string) => {
    intlRouter.replace(pathname, { locale: newLocale });
  };

  const handleLogout = async () => {
    await apiFetch("/api/auth/logout", { method: "POST" }).catch(() => undefined);
    logoutLocal();
    router.push(`/${currentLocale}/login`);
  };

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

  const handleDeleteAccount = async () => {
    setDeletePending(true);
    setDeleteError(null);

    try {
      const res = await apiFetch("/api/me", { method: "DELETE" });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        setDeleteError(
          body?.code && tErr.has(body.code) ? tErr(body.code) : (body?.error ?? t("deleteFailed")),
        );
        return;
      }
      logoutLocal();
      router.push(`/${currentLocale}/signup`);
    } catch {
      setDeleteError(t("deleteFailed"));
    } finally {
      setDeletePending(false);
    }
  };

  const toggleIntegration = (id: string) => {
    if (connectedList.includes(id)) {
      setConnectedList((prev) => prev.filter((x) => x !== id));
    } else {
      setConnectedList((prev) => [...prev, id]);
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-8 p-6">
      <div className="flex flex-col justify-between gap-4 md:flex-row md:items-center">
        <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>

        {/* Horizontal Pill Tabs */}
        <div className="surface-subtle border-border flex items-center gap-1 rounded-md border p-1">
          <button
            onClick={() => setActiveTab("profile")}
            className={`flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
              activeTab === "profile"
                ? "text-text bg-white/[0.07] shadow-sm"
                : "text-muted hover:text-text hover:bg-white/[0.045]"
            }`}
          >
            <Sliders className="h-4 w-4" />
            {t("general")}
          </button>
          <button
            onClick={() => setActiveTab("integrations")}
            className={`flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
              activeTab === "integrations"
                ? "text-text bg-white/[0.07] shadow-sm"
                : "text-muted hover:text-text hover:bg-white/[0.045]"
            }`}
          >
            <Workflow className="h-4 w-4" />
            {t("connectors")}
          </button>
        </div>
      </div>

      <div className="space-y-6">
        {activeTab === "profile" && (
          <>
            {needsStationSetup && (
              <div className="surface border-gold/30 rounded-md p-6 shadow-[inset_0_0_20px_rgba(251,192,45,0.05)]">
                <div className="mb-4 flex items-start gap-3">
                  <ShieldAlert className="text-gold mt-0.5 h-5 w-5 shrink-0" />
                  <div>
                    <h2 className="text-text text-lg font-semibold tracking-tight">
                      {t("setupTitle")}
                    </h2>
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
                  <button
                    type="submit"
                    disabled={createTeam.isPending || !stationName.trim()}
                    className="ow-primary inline-flex h-10 items-center justify-center rounded-md px-6 text-sm font-medium transition-colors disabled:opacity-50"
                  >
                    {createTeam.isPending ? t("creating") : t("createOrganization")}
                  </button>
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
                  <span className="text-muted/70 mb-1 block text-xs font-medium tracking-wider uppercase">
                    {t("emailLabel")}
                  </span>
                  <span className="text-text font-medium">{user?.email ?? t("unknown")}</span>
                </div>
                <div>
                  <span className="text-muted/70 mb-1 block text-xs font-medium tracking-wider uppercase">
                    {t("userId")}
                  </span>
                  <span className="text-text font-mono text-xs">{user?.id ?? t("unknown")}</span>
                </div>
                <div>
                  <span className="text-muted/70 mb-1 block text-xs font-medium tracking-wider uppercase">
                    {t("role")}
                  </span>
                  <span className="text-text font-medium capitalize">
                    {teamsLoading ? t("loading") : (primaryTeam?.role ?? t("noStationYet"))}
                  </span>
                </div>
                <div>
                  <span className="text-muted/70 mb-1 block text-xs font-medium tracking-wider uppercase">
                    {t("organization")}
                  </span>
                  <span className="text-text font-medium">
                    {teamsLoading ? t("loading") : (primaryTeam?.name ?? t("notConfigured"))}
                  </span>
                </div>
              </div>
            </div>

            <div className="surface rounded-md p-6">
              <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
                <Languages className="text-muted h-5 w-5" />
                {t("language")}
              </h2>
              <div className="mt-4 flex items-center justify-between gap-4">
                <div className="min-w-0">
                  <h3 className="text-text text-sm font-medium">{t("interfaceLanguage")}</h3>
                </div>
                <div className="flex shrink-0 gap-4">
                  <button
                    onClick={() => switchLocale("en")}
                    className={`ring-offset-bg overflow-hidden rounded-full ring-offset-2 transition-all ${
                      currentLocale === "en"
                        ? "ring-gold opacity-100 ring-2 grayscale-0"
                        : "opacity-50 grayscale hover:opacity-100 hover:grayscale-0"
                    }`}
                  >
                    <Image
                      src="/assets/en.webp"
                      alt="English"
                      width={24}
                      height={24}
                      className="block object-cover"
                    />
                  </button>
                  <button
                    onClick={() => switchLocale("fr")}
                    className={`ring-offset-bg overflow-hidden rounded-full ring-offset-2 transition-all ${
                      currentLocale === "fr"
                        ? "ring-gold opacity-100 ring-2 grayscale-0"
                        : "opacity-50 grayscale hover:opacity-100 hover:grayscale-0"
                    }`}
                  >
                    <Image
                      src="/assets/fr.webp"
                      alt="Français"
                      width={24}
                      height={24}
                      className="block object-cover"
                    />
                  </button>
                </div>
              </div>
            </div>

            <div className="surface rounded-md p-6">
              <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
                <PencilLine className="text-muted h-5 w-5" />
                {t("accountActions")}
              </h2>
              <div className="mt-4 space-y-4">
                <div className="flex items-center justify-between gap-4">
                  <div className="min-w-0">
                    <h3 className="text-sm font-medium text-red-400">{t("logOutSession")}</h3>
                  </div>
                  <button
                    onClick={handleLogout}
                    className="ow-danger inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50"
                  >
                    <LogOut className="h-4 w-4" />
                    {t("logOut")}
                  </button>
                </div>

                <div className="flex items-center justify-between gap-4">
                  <div className="min-w-0">
                    <h3 className="text-sm font-medium text-red-400">{t("deleteAccountTitle")}</h3>
                  </div>
                  <button
                    onClick={() => setDeleteOpen(true)}
                    className="ow-danger inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50"
                  >
                    <Trash2 className="h-4 w-4" />
                    {t("deleteAccount")}
                  </button>
                </div>
              </div>
            </div>
          </>
        )}

        {activeTab === "integrations" && (
          <div className="surface rounded-md p-6">
            <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
              <Workflow className="text-muted h-5 w-5" />
              {t("connectors")}
            </h2>

            <div className="divide-border mt-2 divide-y">
              {AVAILABLE_INTEGRATIONS.map((integ) => {
                const isActive = connectedList.includes(integ.id);
                return (
                  <div
                    key={integ.id}
                    className="flex items-center justify-between gap-4 py-4 first:pt-2 last:pb-0"
                  >
                    <div className="flex min-w-0 items-center gap-4 pr-4">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center">
                        <Image
                          src={integ.icon}
                          alt={integ.name}
                          width={24}
                          height={24}
                          className="h-7 w-7 object-contain"
                        />
                      </div>
                      <div className="min-w-0 pr-4">
                        <span className="text-text block truncate font-medium">{integ.name}</span>
                        <p className="text-muted/70 mt-0.5 truncate text-sm">{integ.desc}</p>
                      </div>
                    </div>

                    <button
                      type="button"
                      onClick={() => toggleIntegration(integ.id)}
                      className={`inline-flex h-10 shrink-0 items-center justify-center rounded-md px-4 text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50 ${
                        isActive ? "ow-secondary text-text" : "ow-primary"
                      }`}
                    >
                      {isActive ? t("connected") : t("connect")}
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {deleteOpen && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="surface w-full max-w-md space-y-5 rounded-md p-6 shadow-2xl">
            <div className="flex gap-3">
              <AlertTriangle className="text-sev-critical mt-0.5 h-5 w-5 shrink-0" />
              <div>
                <h2 className="text-text text-lg font-semibold">{t("deleteAccount")}</h2>
                <p className="text-muted mt-2 text-sm">
                  {t("deleteModalDesc", { email: user?.email ?? "—" })}
                </p>
              </div>
            </div>
            <input
              value={deleteConfirm}
              onChange={(e) => setDeleteConfirm(e.target.value)}
              className="ow-input focus-visible:ring-sev-critical/50 flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
              placeholder="DELETE"
            />
            {deleteError && <p className="text-sm text-red-400">{deleteError}</p>}
            <div className="flex justify-end gap-3 pt-2">
              <button
                type="button"
                onClick={() => {
                  setDeleteOpen(false);
                  setDeleteConfirm("");
                  setDeleteError(null);
                }}
                className="ow-secondary h-10 rounded-md px-4 text-sm font-medium transition-colors"
              >
                {t("cancel")}
              </button>
              <button
                type="button"
                onClick={handleDeleteAccount}
                disabled={deletePending || deleteConfirm !== "DELETE"}
                className="ow-danger inline-flex h-10 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:opacity-50"
              >
                <Trash2 className="h-4 w-4" />
                {deletePending ? t("deleting") : t("deleteAccount")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
