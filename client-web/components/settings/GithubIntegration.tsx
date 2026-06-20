"use client";

import React, { useState } from "react";
import Image from "next/image";
import { CheckCircle2 } from "lucide-react";
import { useTranslations } from "next-intl";
import {
  useConnectGithub,
  useDisconnectGithub,
  useServiceConnections,
} from "@/lib/queries/serviceConnections";

/**
 * GitHub integration card (Settings > Connectors). Standard provider pattern: a
 * compact status row with Connect / Configure / Disconnect, and the webhook
 * signing secret only inside a configuration panel — masked, then cleared, never
 * shown back. The stored value is a webhook signing secret (HMAC), not a GitHub
 * API token.
 */
export function GithubIntegration() {
  const t = useTranslations("Settings");
  const tErr = useTranslations("errors");
  const { data, isLoading } = useServiceConnections();
  const connect = useConnectGithub();
  const disconnect = useDisconnectGithub();
  const [editing, setEditing] = useState(false);
  const [secret, setSecret] = useState("");
  const [notice, setNotice] = useState<"connected" | "disconnected" | null>(null);

  const connected = data?.github.connected ?? false;

  const closePanel = () => {
    setSecret("");
    setEditing(false);
  };

  const openPanel = () => {
    connect.reset();
    disconnect.reset();
    setNotice(null);
    setEditing(true);
  };

  const togglePanel = () => {
    connect.reset();
    disconnect.reset();
    setNotice(null);
    setEditing((v) => {
      if (v) setSecret("");
      return !v;
    });
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const value = secret.trim();
    if (!value) return;
    connect.reset();
    disconnect.reset();
    setNotice(null);
    connect.mutate(value, {
      onSuccess: () => {
        closePanel();
        setNotice("connected");
      },
    });
  };

  const handleDisconnect = () => {
    connect.reset();
    disconnect.reset();
    closePanel();
    setNotice(null);
    disconnect.mutate(undefined, {
      onSuccess: () => setNotice("disconnected"),
    });
  };

  const failure = connect.error ?? disconnect.error;
  const errorCode = failure instanceof Error ? failure.message : null;
  const errorText = errorCode
    ? tErr.has(errorCode)
      ? tErr(errorCode)
      : t("githubConnectFailed")
    : null;

  return (
    <div className="surface-subtle border-border rounded-md border p-4">
      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <Image
            src="/assets/github-patched.webp"
            alt="GitHub"
            width={24}
            height={24}
            className="h-7 w-7 shrink-0 object-contain"
          />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-text truncate font-medium">{t("githubTitle")}</span>
              <span className="border-border text-muted shrink-0 rounded border px-1.5 py-0.5 text-[10px] font-medium">
                Webhook
              </span>
            </div>
            <p className="text-muted/70 truncate text-xs">{t("githubDesc")}</p>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-3">
          {isLoading ? (
            <span className="text-muted text-xs">{t("loading")}</span>
          ) : connected ? (
            <span className="text-st-res inline-flex items-center gap-1.5 text-xs font-medium">
              <CheckCircle2 className="h-4 w-4" />
              {t("connected")}
            </span>
          ) : (
            <span className="text-muted/70 text-xs">{t("notConnected")}</span>
          )}

          {!isLoading &&
            (connected ? (
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={togglePanel}
                  className="ow-secondary text-text inline-flex h-9 items-center rounded-md px-3 text-sm font-medium"
                >
                  {t("configure")}
                </button>
                <button
                  type="button"
                  onClick={handleDisconnect}
                  disabled={disconnect.isPending}
                  className="ow-danger inline-flex h-9 items-center rounded-md px-3 text-sm font-medium disabled:opacity-50"
                >
                  {disconnect.isPending ? t("disconnecting") : t("disconnect")}
                </button>
              </div>
            ) : (
              <button
                type="button"
                onClick={openPanel}
                className="ow-primary inline-flex h-9 items-center rounded-md px-4 text-sm font-medium"
              >
                {t("connect")}
              </button>
            ))}
        </div>
      </div>

      {editing && (
        <form onSubmit={handleSubmit} className="border-border mt-4 border-t pt-4">
          <label
            htmlFor="github-webhook-secret"
            className="text-muted mb-1.5 block text-xs font-medium"
          >
            {t("githubSecretLabel")}
          </label>
          <div className="flex flex-col gap-2 sm:flex-row">
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
              {connect.isPending ? t("saving") : t("save")}
            </button>
            <button
              type="button"
              onClick={closePanel}
              className="ow-secondary text-text inline-flex h-10 shrink-0 items-center justify-center rounded-md px-4 text-sm font-medium"
            >
              {t("cancel")}
            </button>
          </div>
          <p className="text-muted/60 mt-2 text-xs">{t("githubSecretHint")}</p>
        </form>
      )}

      {!editing && notice === "connected" && !errorText && (
        <p className="text-st-res mt-2 text-xs">{t("githubConnected")}</p>
      )}
      {!editing && notice === "disconnected" && !errorText && (
        <p className="text-muted mt-2 text-xs">{t("githubDisconnected")}</p>
      )}
      {errorText && <p className="mt-2 text-xs text-red-400">{errorText}</p>}
    </div>
  );
}
