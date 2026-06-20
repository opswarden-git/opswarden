"use client";

import React, { useState } from "react";
import Image from "next/image";
import { CheckCircle2 } from "lucide-react";
import { useTranslations } from "next-intl";
import { useConnectGithub, useServiceConnections } from "@/lib/queries/serviceConnections";

/**
 * GitHub integration row wired to the real backend
 * (`GET/PUT /api/service-connections`). The webhook secret is write-only: it is
 * masked while typing, sent to the vault, then cleared — never displayed back.
 */
export function GithubIntegration() {
  const t = useTranslations("Settings");
  const tErr = useTranslations("errors");
  const { data, isLoading } = useServiceConnections();
  const connect = useConnectGithub();
  const [secret, setSecret] = useState("");

  const connected = data?.github.connected ?? false;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const value = secret.trim();
    if (!value) return;
    connect.mutate(value, {
      // Never keep the secret in memory/UI after a successful save.
      onSuccess: () => setSecret(""),
    });
  };

  const errorCode = connect.error instanceof Error ? connect.error.message : null;
  const errorText = errorCode
    ? tErr.has(errorCode)
      ? tErr(errorCode)
      : t("githubConnectFailed")
    : null;

  return (
    <div className="py-4 first:pt-2">
      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-4 pr-4">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center">
            <Image
              src="/assets/github-patched.webp"
              alt="GitHub"
              width={24}
              height={24}
              className="h-7 w-7 object-contain"
            />
          </div>
          <div className="min-w-0 pr-4">
            <span className="text-text block truncate font-medium">GitHub</span>
            <p className="text-muted/70 mt-0.5 truncate text-sm">{t("githubDesc")}</p>
          </div>
        </div>

        {isLoading ? (
          <span className="text-muted shrink-0 text-xs">{t("loading")}</span>
        ) : connected ? (
          <span className="text-st-res inline-flex shrink-0 items-center gap-1.5 text-xs font-medium">
            <CheckCircle2 className="h-4 w-4" />
            {t("connected")}
          </span>
        ) : (
          <span className="text-muted/70 shrink-0 text-xs">{t("notConnected")}</span>
        )}
      </div>

      <form onSubmit={handleSubmit} className="mt-4 flex flex-col gap-2 sm:flex-row">
        <label htmlFor="github-webhook-secret" className="sr-only">
          {t("githubSecretLabel")}
        </label>
        <input
          id="github-webhook-secret"
          type="password"
          autoComplete="off"
          value={secret}
          onChange={(e) => setSecret(e.target.value)}
          placeholder={t("githubSecretPlaceholder")}
          className="ow-input flex h-10 min-w-0 flex-1 rounded-md px-3 py-2 text-sm transition-colors"
        />
        <button
          type="submit"
          disabled={connect.isPending || !secret.trim()}
          className="ow-primary inline-flex h-10 shrink-0 items-center justify-center rounded-md px-6 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
        >
          {connect.isPending ? t("saving") : connected ? t("update") : t("connect")}
        </button>
      </form>

      {connect.isSuccess && !errorText && (
        <p className="text-st-res mt-2 text-xs">{t("githubConnected")}</p>
      )}
      {errorText && <p className="mt-2 text-xs text-red-400">{errorText}</p>}
      <p className="text-muted/60 mt-2 text-xs">{t("githubSecretHint")}</p>
    </div>
  );
}
