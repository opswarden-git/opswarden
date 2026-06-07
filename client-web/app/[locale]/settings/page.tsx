"use client";

import React, { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import {
  LogOut,
  Trash2,
  UserRound,
  Settings,
  Sliders,
  Workflow,
  Languages,
  PencilLine,
} from "lucide-react";
import Image from "next/image";
import { useRouter as useIntlRouter, usePathname } from "../../../i18n/routing";

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
  const [activeTab, setActiveTab] = useState<"profile" | "integrations">("profile");
  const [connectedList, setConnectedList] = useState<string[]>(["github", "k8s"]);

  const intlRouter = useIntlRouter();
  const pathname = usePathname();
  const params = useParams();
  const currentLocale = params.locale as string;

  const switchLocale = (newLocale: string) => {
    intlRouter.replace(pathname, { locale: newLocale });
  };

  const handleLogout = () => {
    router.push("/login");
  };

  const handleDeleteAccount = () => {
    if (
      confirm(
        "WARNING: Are you sure you want to permanently delete this operator account? This action is irreversible.",
      )
    ) {
      router.push("/signup");
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
        {/* Left Column: Title & Navigation */}
        <div className="space-y-8">
          <div>
            <h1 className="text-text flex items-center gap-4 text-4xl font-bold tracking-tight">
              Settings
            </h1>
          </div>

          <div className="space-y-4 font-mono text-base">
            <button
              onClick={() => setActiveTab("profile")}
              className={`flex w-full items-center gap-4 py-2 text-left transition-colors ${
                activeTab === "profile" ? "text-gold font-bold" : "text-muted hover:text-text"
              }`}
            >
              <Sliders className="h-5 w-5" />
              General
            </button>
            <button
              onClick={() => setActiveTab("integrations")}
              className={`flex w-full items-center gap-4 py-2 text-left transition-colors ${
                activeTab === "integrations" ? "text-gold font-bold" : "text-muted hover:text-text"
              }`}
            >
              <Workflow className="h-5 w-5" />
              Connectors
            </button>
          </div>
        </div>

        {/* Content Area */}
        <div className="space-y-6 md:col-span-3">
          {activeTab === "profile" && (
            <>
              {/* Mock Operator Profile Section */}
              <div className="glass space-y-4 rounded-lg p-8">
                <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 font-mono text-xl font-bold">
                  <UserRound className="text-muted h-6 w-6" />
                  User
                </h2>
                <div className="grid grid-cols-2 gap-4 font-mono text-base">
                  <div>
                    <span className="text-muted mb-1 block text-sm uppercase">Operator Alias</span>
                    <span className="text-text font-bold">Operator Alpha</span>
                  </div>
                  <div>
                    <span className="text-muted mb-1 block text-sm uppercase">Clearance Level</span>
                    <span className="text-text font-bold">Level 1 NOC</span>
                  </div>
                  <div className="col-span-2">
                    <span className="text-muted mb-1 block text-sm uppercase">Active Station</span>
                    <span className="text-text">Core NOC Paris</span>
                  </div>
                </div>
              </div>

              {/* Regional Preferences Section */}
              <div className="glass space-y-4 rounded-lg p-8">
                <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 font-mono text-xl font-bold">
                  <Languages className="text-muted h-6 w-6" />
                  Language
                </h2>
                <div className="flex items-center justify-between gap-4 p-4">
                  <div className="min-w-0">
                    <h3 className="text-text text-base font-bold">Interface Language</h3>
                  </div>
                  <div className="flex shrink-0 gap-4">
                    <button
                      onClick={() => switchLocale("en")}
                      className={`font-mono text-sm transition-colors ${
                        currentLocale === "en"
                          ? "text-gold font-bold"
                          : "text-muted hover:text-text"
                      }`}
                    >
                      EN
                    </button>
                    <button
                      onClick={() => switchLocale("fr")}
                      className={`font-mono text-sm transition-colors ${
                        currentLocale === "fr"
                          ? "text-gold font-bold"
                          : "text-muted hover:text-text"
                      }`}
                    >
                      FR
                    </button>
                  </div>
                </div>
              </div>

              {/* Account Actions Section */}
              <div className="glass space-y-6 rounded-lg p-8">
                <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 font-mono text-xl font-bold">
                  <PencilLine className="text-muted h-6 w-6" />
                  Account Actions
                </h2>

                <div className="space-y-0 font-mono">
                  {/* Log Out Option */}
                  <div className="flex items-center justify-between gap-4 p-4">
                    <div className="min-w-0">
                      <h3 className="text-base font-bold text-red-400">Log Out Session</h3>
                    </div>
                    <button
                      onClick={handleLogout}
                      className="flex shrink-0 items-center gap-2 rounded border-none bg-red-600 px-4 py-2.5 text-sm font-bold tracking-wider text-white uppercase transition-all hover:bg-red-700"
                    >
                      <LogOut className="h-5 w-5" />
                      Log Out
                    </button>
                  </div>

                  {/* Delete Account Option */}
                  <div className="flex items-center justify-between gap-4 p-4">
                    <div className="min-w-0">
                      <h3 className="text-base font-bold text-red-400">Delete Account</h3>
                    </div>
                    <button
                      onClick={handleDeleteAccount}
                      className="flex shrink-0 items-center gap-2 rounded border-none bg-red-600 px-4 py-2.5 text-sm font-bold tracking-wider text-white uppercase transition-all hover:bg-red-700"
                    >
                      <Trash2 className="h-5 w-5" />
                      Delete Account
                    </button>
                  </div>
                </div>
              </div>
            </>
          )}

          {activeTab === "integrations" && (
            <div className="glass space-y-6 rounded-lg p-8">
              <h2 className="text-text flex items-center gap-2 border-b border-white/5 pb-3 font-mono text-xl font-bold">
                <Workflow className="text-muted h-6 w-6" />
                Connectors
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
                          <span className="text-text block truncate font-mono text-base font-bold">
                            {integ.name}
                          </span>
                          <p className="text-muted mt-0.5 truncate text-sm">{integ.desc}</p>
                        </div>
                      </div>

                      <button
                        type="button"
                        onClick={() => toggleIntegration(integ.id)}
                        className={`shrink-0 rounded px-4 py-2 font-mono text-sm font-bold uppercase transition-all ${
                          isActive
                            ? "text-muted hover:text-text bg-white/5 hover:bg-white/10"
                            : "hover:bg-gold-hover bg-gold text-bg"
                        }`}
                      >
                        {isActive ? "Connected" : "Connect"}
                      </button>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
