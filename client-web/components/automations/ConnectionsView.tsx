"use client";

import { CheckCircle2, GitBranch, Globe2, Pencil, Plug, Send, Trash2 } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import React, { useMemo, useRef, useState, useSyncExternalStore } from "react";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { CopyButton } from "@/components/ui/CopyButton";
import { automationWebhookUrl } from "@/lib/automation-routing";
import {
  type AutomationRule,
  type AutomationService,
  type TeamConnection,
  useConfigureGithubConnection,
  useConfigureHttpConnection,
  useDeleteTeamConnection,
  useTestTeamConnection,
} from "@/lib/queries/automations";
import { AutomationDialog } from "./AutomationDialog";

function connectedServiceNames(catalog: AutomationService[]) {
  return new Set(
    catalog.flatMap((service) =>
      [...service.actions, ...service.reactions].flatMap((capability) =>
        capability.connection_service ? [capability.connection_service] : [],
      ),
    ),
  );
}

function ConnectionStatus({ connection }: { connection?: TeamConnection }) {
  const t = useTranslations("Automations");
  if (!connection) return <span className="text-muted text-xs">{t("notConfigured")}</span>;
  if (connection.last_error_code) {
    return (
      <span className="text-sev-critical inline-flex items-center gap-1.5 text-xs font-medium">
        <span className="bg-sev-critical h-1.5 w-1.5 rounded-full" />
        {t("needsAttention")}
      </span>
    );
  }
  if (connection.verified_at || connection.last_delivery_at) {
    return (
      <span className="text-st-res inline-flex items-center gap-1.5 text-xs font-medium">
        <CheckCircle2 className="h-3.5 w-3.5" aria-hidden="true" />
        {t("verified")}
      </span>
    );
  }
  return (
    <span className="text-sev-medium inline-flex items-center gap-1.5 text-xs font-medium">
      <span className="bg-sev-medium h-1.5 w-1.5 rounded-full" />
      {t("awaitingVerification")}
    </span>
  );
}

function GithubForm({
  connection,
  onClose,
  teamId,
}: {
  connection?: TeamConnection;
  onClose: () => void;
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const secretRef = useRef<HTMLInputElement>(null);
  const [secret, setSecret] = useState("");
  const [token, setToken] = useState("");
  const configure = useConfigureGithubConnection(teamId);
  const valid = !!connection || secret.trim().length > 0;

  return (
    <AutomationDialog
      open
      onClose={onClose}
      initialFocus={secretRef}
      title={connection ? t("reconfigureGithub") : t("configureGithub")}
      description={t("githubFormDescription")}
    >
      <form
        className="min-h-0 space-y-5 overflow-y-auto p-6"
        onSubmit={(event) => {
          event.preventDefault();
          if (!valid) return;
          configure.mutate(
            {
              ...(secret.trim() ? { webhook_signing_secret: secret.trim() } : {}),
              ...(token.trim() ? { personal_token: token.trim() } : {}),
            },
            { onSuccess: onClose },
          );
        }}
      >
        <label className="text-text block text-sm font-medium">
          <span>{t("signingSecret")}</span>
          <input
            ref={secretRef}
            type="password"
            value={secret}
            onChange={(event) => setSecret(event.target.value)}
            className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            autoComplete="new-password"
            required={!connection}
          />
          <span className="text-muted mt-1.5 block text-xs">
            {connection ? t("secretPreservedHint") : t("secretRequiredHint")}
          </span>
        </label>
        <label className="text-text block text-sm font-medium">
          <span>{t("personalTokenOptional")}</span>
          <input
            type="password"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            autoComplete="new-password"
          />
          <span className="text-muted mt-1.5 block text-xs">{t("tokenHint")}</span>
        </label>
        {configure.error ? (
          <Alert tone="danger">{t("requestFailed", { code: configure.error.message })}</Alert>
        ) : null}
        <div className="border-border flex justify-end gap-2 border-t pt-5">
          <Button size="lg" onClick={onClose}>
            {t("cancel")}
          </Button>
          <Button
            type="submit"
            size="lg"
            variant="primary"
            disabled={!valid}
            loading={configure.isPending}
          >
            {t("saveConnection")}
          </Button>
        </div>
      </form>
    </AutomationDialog>
  );
}

function HttpForm({
  connection,
  onClose,
  teamId,
}: {
  connection?: TeamConnection;
  onClose: () => void;
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const inputRef = useRef<HTMLInputElement>(null);
  const [endpoint, setEndpoint] = useState("");
  const configure = useConfigureHttpConnection(teamId);

  return (
    <AutomationDialog
      open
      onClose={onClose}
      initialFocus={inputRef}
      title={connection ? t("reconfigureHttp") : t("configureHttp")}
      description={t("httpFormDescription")}
    >
      <form
        className="min-h-0 space-y-5 overflow-y-auto p-6"
        onSubmit={(event) => {
          event.preventDefault();
          if (!endpoint.trim()) return;
          configure.mutate(endpoint.trim(), { onSuccess: onClose });
        }}
      >
        <label className="text-text block text-sm font-medium">
          <span>{t("endpointUrl")}</span>
          <input
            ref={inputRef}
            type="url"
            value={endpoint}
            onChange={(event) => setEndpoint(event.target.value)}
            className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            placeholder="https://hooks.example.com/opswarden"
            required
          />
          <span className="text-muted mt-1.5 block text-xs">{t("endpointSecurityHint")}</span>
        </label>
        {configure.error ? (
          <Alert tone="danger">{t("requestFailed", { code: configure.error.message })}</Alert>
        ) : null}
        <div className="border-border flex justify-end gap-2 border-t pt-5">
          <Button size="lg" onClick={onClose}>
            {t("cancel")}
          </Button>
          <Button
            type="submit"
            size="lg"
            variant="primary"
            disabled={!endpoint.trim()}
            loading={configure.isPending}
          >
            {t("saveConnection")}
          </Button>
        </div>
      </form>
    </AutomationDialog>
  );
}

export function ConnectionsView({
  catalog,
  connections,
  rules,
  teamId,
}: {
  catalog: AutomationService[];
  connections: TeamConnection[];
  rules: AutomationRule[];
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const locale = useLocale();
  const [editing, setEditing] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<TeamConnection | null>(null);
  const testConnection = useTestTeamConnection(teamId);
  const deleteConnection = useDeleteTeamConnection(teamId);
  const connectionServices = useMemo(() => connectedServiceNames(catalog), [catalog]);
  const services = catalog.filter((service) => connectionServices.has(service.name));
  const browserOrigin = useSyncExternalStore(
    () => () => undefined,
    () => window.location.origin,
    () => "",
  );
  const webhookUrls = Object.fromEntries(
    connections.flatMap((connection) =>
      connection.webhook_path && browserOrigin
        ? [[connection.id, automationWebhookUrl(connection.webhook_path, browserOrigin)]]
        : [],
    ),
  );

  return (
    <>
      <div className="grid gap-4 lg:grid-cols-2">
        {services.map((service) => {
          const connection = connections.find((item) => item.service === service.name);
          const usedBy = connection
            ? rules.filter(
                (rule) =>
                  rule.trigger_connection_id === connection.id ||
                  rule.reaction_connection_id === connection.id,
              ).length
            : 0;
          const Icon = service.name === "github" ? GitBranch : Globe2;
          return (
            <section key={service.name} className="surface flex min-h-64 flex-col rounded-md">
              <div className="border-border flex items-start gap-4 border-b p-5">
                <div className="surface-subtle text-text flex h-10 w-10 shrink-0 items-center justify-center rounded-md">
                  <Icon className="h-5 w-5" aria-hidden="true" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <h3 className="text-text font-semibold">{service.label}</h3>
                    <ConnectionStatus connection={connection} />
                  </div>
                  <p className="text-muted mt-1 text-sm">
                    {service.actions[0]?.description ?? service.reactions[0]?.description}
                  </p>
                </div>
              </div>

              <div className="min-h-0 flex-1 space-y-3 p-5">
                {connection?.webhook_path ? (
                  <div>
                    <div className="text-muted mb-1.5 text-xs font-medium uppercase">
                      {t("webhookUrl")}
                    </div>
                    <div className="surface-subtle border-border flex items-center gap-2 rounded-md border p-2">
                      <code className="text-text min-w-0 flex-1 truncate text-xs">
                        {webhookUrls[connection.id] ?? connection.webhook_path}
                      </code>
                      <CopyButton
                        value={webhookUrls[connection.id] ?? connection.webhook_path}
                        label={t("copyWebhookUrl")}
                        copiedLabel={t("copied")}
                        size="sm"
                        variant="ghost"
                      />
                    </div>
                  </div>
                ) : null}
                {connection ? (
                  <dl className="grid grid-cols-2 gap-3 text-xs">
                    <div>
                      <dt className="text-muted">{t("lastActivity")}</dt>
                      <dd className="text-text mt-1">
                        {connection.last_delivery_at || connection.verified_at
                          ? new Intl.DateTimeFormat(locale, {
                              dateStyle: "medium",
                              timeStyle: "short",
                            }).format(
                              new Date(connection.last_delivery_at ?? connection.verified_at!),
                            )
                          : t("never")}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-muted">{t("usedByRules")}</dt>
                      <dd className="text-text mt-1 tabular-nums">{usedBy}</dd>
                    </div>
                  </dl>
                ) : (
                  <p className="text-muted text-sm">{t("connectionEmpty")}</p>
                )}
                {connection?.last_error_code ? (
                  <Alert tone="danger">
                    {t("lastError", { code: connection.last_error_code })}
                  </Alert>
                ) : null}
                {testConnection.error && testConnection.variables === connection?.id ? (
                  <Alert tone="danger">
                    {t("requestFailed", { code: testConnection.error.message })}
                  </Alert>
                ) : null}
                {testConnection.isSuccess && testConnection.variables === connection?.id ? (
                  <Alert tone="success">{t("testSucceeded")}</Alert>
                ) : null}
              </div>

              <div className="border-border flex flex-wrap justify-end gap-2 border-t p-4">
                {connection?.service === "http" ? (
                  <Button
                    size="sm"
                    onClick={() => testConnection.mutate(connection.id)}
                    loading={testConnection.isPending && testConnection.variables === connection.id}
                  >
                    <Send className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("test")}
                  </Button>
                ) : null}
                {connection ? (
                  <Button
                    size="sm"
                    onClick={() => setDeleting(connection)}
                    disabled={usedBy > 0}
                    title={usedBy > 0 ? t("connectionInUse", { count: usedBy }) : undefined}
                  >
                    <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("disconnect")}
                  </Button>
                ) : null}
                <Button
                  size="sm"
                  variant={connection ? "secondary" : "primary"}
                  onClick={() => setEditing(service.name)}
                >
                  {connection ? (
                    <Pencil className="h-3.5 w-3.5" />
                  ) : (
                    <Plug className="h-3.5 w-3.5" />
                  )}
                  {connection ? t("configure") : t("connect")}
                </Button>
              </div>
            </section>
          );
        })}
      </div>

      {editing === "github" ? (
        <GithubForm
          teamId={teamId}
          connection={connections.find((item) => item.service === "github")}
          onClose={() => setEditing(null)}
        />
      ) : null}
      {editing === "http" ? (
        <HttpForm
          teamId={teamId}
          connection={connections.find((item) => item.service === "http")}
          onClose={() => setEditing(null)}
        />
      ) : null}
      <ConfirmDialog
        open={!!deleting}
        title={t("disconnectTitle", { service: deleting?.service ?? "" })}
        description={t("disconnectDescription")}
        confirmLabel={t("disconnect")}
        cancelLabel={t("cancel")}
        danger
        pending={deleteConnection.isPending}
        error={
          deleteConnection.error
            ? t("requestFailed", { code: deleteConnection.error.message })
            : null
        }
        onClose={() => setDeleting(null)}
        onConfirm={() =>
          deleting && deleteConnection.mutate(deleting.id, { onSuccess: () => setDeleting(null) })
        }
      />
    </>
  );
}
