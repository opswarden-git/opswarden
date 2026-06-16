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
    <div className="w-full max-w-7xl pl-[3px]">
      <div className="grid grid-cols-1 gap-8 md:grid-cols-4">
        <div className="space-y-8">
          <div>
            <h1 className="text-text flex items-center gap-4 text-4xl font-bold tracking-tight">
              {t("title")}
            </h1>
          </div>

          <div className="space-y-4 text-sm font-medium">
            <button
              onClick={() => setActiveTab("profile")}
              className={`flex w-full items-center gap-4 py-2 text-left capitalize transition-colors ${
                activeTab === "profile" ? "text-gold font-bold" : "text-muted hover:text-text"
              }`}
            >
              <Sliders className="h-5 w-5" />
              {t("general")}
            </button>
            <button
              onClick={() => setActiveTab("integrations")}
              className={`flex w-full items-center gap-4 py-2 text-left capitalize transition-colors ${
                activeTab === "integrations" ? "text-gold font-bold" : "text-muted hover:text-text"
              }`}
            >
              <Workflow className="h-5 w-5" />
              {t("connectors")}
            </button>
          </div>
        </div>

        <div className="space-y-6 md:col-span-3">
          {activeTab === "profile" && (
            <>
              {needsStationSetup && (
                <div className="border-gold/30 bg-gold/5 space-y-4 rounded-lg border p-6">
                  <div className="flex items-start gap-3">
                    <ShieldAlert className="text-gold mt-0.5 h-5 w-5 shrink-0" />
                    <div>
                      <h2 className="text-text text-lg font-semibold tracking-tight">
                        {t("setupTitle")}
                      </h2>
                      <p className="text-muted mt-1 text-sm">{t("setupDesc")}</p>
                    </div>
                  </div>
                  <form onSubmit={handleCreateStation} className="flex flex-col gap-3 sm:flex-row">
                    <input
                      type="text"
                      value={stationName}
                      onChange={(e) => setStationName(e.target.value)}
                      placeholder={t("organization")}
                      className="bg-bg border-border text-text placeholder:text-muted-2 focus-visible:ring-gold h-10 min-w-0 flex-1 rounded-md border px-3 text-sm focus-visible:ring-1 focus-visible:outline-none"
                    />
                    <button
                      type="submit"
                      disabled={createTeam.isPending || !stationName.trim()}
                      className="bg-gold text-bg hover:bg-gold/90 focus-visible:ring-gold inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-[#121317] focus-visible:outline-none disabled:opacity-50"
                    >
                      {createTeam.isPending ? t("creating") : t("createOrganization")}
                    </button>
                  </form>
                  {createTeam.isError && (
                    <p className="text-sm text-red-500">{createTeam.error.message}</p>
                  )}
                </div>
              )}

              <div className="glass space-y-4 rounded-lg p-8">
                <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 text-xl font-semibold tracking-tight">
                  <UserRound className="text-muted h-6 w-6" />
                  {t("user")}
                </h2>
                <div className="grid grid-cols-1 gap-4 text-sm sm:grid-cols-2">
                  <div>
                    <span className="text-muted mb-1 block text-xs font-medium">
                      {t("emailLabel")}
                    </span>
                    <span className="text-text font-semibold">{user?.email ?? t("unknown")}</span>
                  </div>
                  <div>
                    <span className="text-muted mb-1 block text-xs font-medium">{t("userId")}</span>
                    <span className="text-text font-mono text-xs">{user?.id ?? t("unknown")}</span>
                  </div>
                  <div>
                    <span className="text-muted mb-1 block text-xs font-medium">{t("role")}</span>
                    <span className="text-text font-semibold capitalize">
                      {teamsLoading ? t("loading") : (primaryTeam?.role ?? t("noStationYet"))}
                    </span>
                  </div>
                  <div>
                    <span className="text-muted mb-1 block text-xs font-medium">
                      {t("organization")}
                    </span>
                    <span className="text-text font-semibold">
                      {teamsLoading ? t("loading") : (primaryTeam?.name ?? t("notConfigured"))}
                    </span>
                  </div>
                </div>
              </div>

              <div className="glass space-y-4 rounded-lg p-8">
                <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 text-xl font-semibold tracking-tight">
                  <Languages className="text-muted h-6 w-6" />
                  {t("language")}
                </h2>
                <div className="flex items-center justify-between gap-4 p-4">
                  <div className="min-w-0">
                    <h3 className="text-text text-sm font-medium">{t("interfaceLanguage")}</h3>
                  </div>
                  <div className="flex shrink-0 gap-4">
                    <button
                      onClick={() => switchLocale("en")}
                      className={`overflow-hidden rounded-full transition-all ${
                        currentLocale === "en" ? "opacity-100 grayscale-0" : "opacity-50 grayscale hover:opacity-100 hover:grayscale-0"
                      }`}
                    >
                      <Image src="/assets/en.webp" alt="English" width={24} height={24} className="block object-cover" />
                    </button>
                    <button
                      onClick={() => switchLocale("fr")}
                      className={`overflow-hidden rounded-full transition-all ${
                        currentLocale === "fr" ? "opacity-100 grayscale-0" : "opacity-50 grayscale hover:opacity-100 hover:grayscale-0"
                      }`}
                    >
                      <Image src="/assets/fr.webp" alt="Français" width={24} height={24} className="block object-cover" />
                    </button>
                  </div>
                </div>
              </div>

              <div className="glass space-y-6 rounded-lg p-8">
                <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 text-xl font-semibold tracking-tight">
                  <PencilLine className="text-muted h-6 w-6" />
                  {t("accountActions")}
                </h2>

                <div className="space-y-0">
                  <div className="flex items-center justify-between gap-4 p-4">
                    <div className="min-w-0">
                      <h3 className="text-sm font-medium text-red-500">{t("logOutSession")}</h3>
                    </div>
                    <button
                      onClick={handleLogout}
                      className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md bg-red-600 px-4 py-2 text-sm font-medium whitespace-nowrap text-white transition-colors hover:bg-red-700 focus-visible:ring-2 focus-visible:ring-red-600 focus-visible:ring-offset-2 focus-visible:ring-offset-[#121317] focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
                    >
                      <LogOut className="h-4 w-4" />
                      {t("logOut")}
                    </button>
                  </div>

                  <div className="flex items-center justify-between gap-4 p-4">
                    <div className="min-w-0">
                      <h3 className="text-sm font-medium text-red-500">
                        {t("deleteAccountTitle")}
                      </h3>
                    </div>
                    <button
                      onClick={() => setDeleteOpen(true)}
                      className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md bg-red-600 px-4 py-2 text-sm font-medium whitespace-nowrap text-white transition-colors hover:bg-red-700 focus-visible:ring-2 focus-visible:ring-red-600 focus-visible:ring-offset-2 focus-visible:ring-offset-[#121317] focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
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
            <div className="glass space-y-6 rounded-lg p-8">
              <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 text-xl font-semibold tracking-tight">
                <Workflow className="text-muted h-6 w-6" />
                {t("connectors")}
              </h2>

              <div className="space-y-4">
                {AVAILABLE_INTEGRATIONS.map((integ) => {
                  const isActive = connectedList.includes(integ.id);
                  return (
                    <div
                      key={integ.id}
                      className="flex items-center justify-between rounded-lg p-4 transition-colors hover:bg-white/5"
                    >
                      <div className="flex min-w-0 items-center gap-4 pr-4">
                        <div className="flex shrink-0 items-center justify-center">
                          <Image
                            src={integ.icon}
                            alt={integ.name}
                            width={24}
                            height={24}
                            className="h-8 w-8 object-contain"
                          />
                        </div>
                        <div className="min-w-0 pr-4">
                          <span className="text-text block truncate text-sm font-medium">
                            {integ.name}
                          </span>
                          <p className="text-muted mt-0.5 truncate text-xs">{integ.desc}</p>
                        </div>
                      </div>

                      <button
                        type="button"
                        onClick={() => toggleIntegration(integ.id)}
                        className={`inline-flex h-9 shrink-0 items-center justify-center rounded-md px-4 py-2 text-sm font-medium whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:ring-white/20 focus-visible:ring-offset-2 focus-visible:ring-offset-[#121317] focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50 ${
                          isActive
                            ? "text-text bg-[#1b1b20] hover:bg-border"
                            : "bg-gold hover:bg-gold/90 text-[#1a1405]"
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
      </div>

      {deleteOpen && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="glass w-full max-w-md space-y-5 rounded-lg border border-red-500/20 p-6">
            <div className="flex gap-3">
              <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-red-500" />
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
              className="bg-bg border-border text-text placeholder:text-muted-2 focus-visible:ring-gold h-10 w-full rounded-md border px-3 text-sm focus-visible:ring-1 focus-visible:outline-none"
              placeholder="DELETE"
            />
            {deleteError && <p className="text-sm text-red-500">{deleteError}</p>}
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={() => {
                  setDeleteOpen(false);
                  setDeleteConfirm("");
                  setDeleteError(null);
                }}
                className="text-muted hover:text-text h-9 rounded-md px-4 text-sm font-medium transition-colors"
              >
                {t("cancel")}
              </button>
              <button
                type="button"
                onClick={handleDeleteAccount}
                disabled={deletePending || deleteConfirm !== "DELETE"}
                className="inline-flex h-9 items-center justify-center gap-2 rounded-md bg-red-600 px-4 text-sm font-medium text-white transition-colors hover:bg-red-700 disabled:opacity-50"
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
