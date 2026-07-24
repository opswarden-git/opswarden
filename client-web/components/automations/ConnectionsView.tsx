"use client";

import { CheckCircle2, Globe2, Pencil, Plug, Send, Trash2 } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import React, { useRef, useState, useSyncExternalStore } from "react";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { CopyButton } from "@/components/ui/CopyButton";
import { automationWebhookUrl } from "@/lib/automation-routing";
import {
  catalogFieldsAreValid,
  catalogPayload,
  catalogValues,
  connectableServices,
} from "@/lib/automation-catalog";
import {
  type AutomationRule,
  type AutomationService,
  type TeamConnection,
  useConfigureTeamConnection,
  useDeleteTeamConnection,
  useRefreshServiceOAuth,
  useStartServiceOAuth,
  useTestTeamConnection,
} from "@/lib/queries/automations";
import { AutomationDialog } from "./AutomationDialog";
import { FormField } from "@/components/ui/FormField";

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

function ConnectionForm({
  connection,
  onClose,
  service,
  teamId,
}: {
  connection?: TeamConnection;
  onClose: () => void;
  service: AutomationService;
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const locale = useLocale();
  const inputRef = useRef<HTMLInputElement>(null);
  const fields = service.connection?.fields ?? [];
  const [values, setValues] = useState(() => catalogValues(fields));
  const configure = useConfigureTeamConnection(teamId);
  const startOAuth = useStartServiceOAuth(teamId);
  const valid = catalogFieldsAreValid(fields, values, !!connection);

  return (
    <AutomationDialog
      open
      onClose={onClose}
      initialFocus={inputRef}
      title={
        connection
          ? t("reconfigureService", { service: service.label })
          : t("connectService", { service: service.label })
      }
      description={service.connection?.description ?? service.label}
    >
      <form
        className="min-h-0 space-y-5 overflow-y-auto p-6"
        onSubmit={(event) => {
          event.preventDefault();
          if (!valid) return;
          configure.mutate(
            {
              service: service.name,
              payload: catalogPayload(fields, values),
            },
            { onSuccess: onClose },
          );
        }}
      >
        {service.connection?.oauth ? (
          <div className="surface-subtle border-border space-y-3 rounded-md border p-4">
            <div>
              <p className="text-text text-sm font-medium">{service.connection.oauth.label}</p>
              <p className="text-muted mt-1 text-xs">{service.connection.oauth.description}</p>
            </div>
            <Button
              type="button"
              size="sm"
              variant="secondary"
              loading={startOAuth.isPending}
              onClick={() =>
                startOAuth.mutate(
                  { locale, service: service.name },
                  {
                    onSuccess: ({ authorization_url }) => window.location.assign(authorization_url),
                  },
                )
              }
            >
              <Plug className="h-3.5 w-3.5" aria-hidden="true" />
              {service.connection.oauth.label}
            </Button>
            {startOAuth.error ? (
              <Alert tone="danger">{t("requestFailed", { code: startOAuth.error.message })}</Alert>
            ) : null}
          </div>
        ) : null}
        {service.connection?.oauth && fields.length > 0 ? (
          <div className="border-border border-t pt-5">
            <p className="text-muted text-xs">{t("manualConnectionAlternative")}</p>
          </div>
        ) : null}
        {fields.map((field, index) => (
          <FormField
            key={field.name}
            label={field.label}
            caption={
              connection && field.required
                ? `${field.description} ${t("blankPreservesExisting")}`
                : field.description
            }
            required={field.required && !connection}
          >
            {field.input_type === "select" ? (
              <select
                value={values[field.name] ?? ""}
                onChange={(event) =>
                  setValues((current) => ({ ...current, [field.name]: event.target.value }))
                }
                className="ow-input h-10 w-full rounded-md px-3 text-sm"
              >
                {field.options.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            ) : (
              <input
                ref={index === 0 ? inputRef : undefined}
                type={field.input_type}
                value={values[field.name] ?? ""}
                onChange={(event) =>
                  setValues((current) => ({ ...current, [field.name]: event.target.value }))
                }
                className="ow-input h-10 w-full rounded-md px-3 text-sm"
                autoComplete={field.input_type === "password" ? "new-password" : undefined}
              />
            )}
          </FormField>
        ))}
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
  const refreshOAuth = useRefreshServiceOAuth(teamId);
  const deleteConnection = useDeleteTeamConnection(teamId);
  const services = connectableServices(catalog);
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
          return (
            <section key={service.name} className="surface flex min-h-64 flex-col rounded-md">
              <div className="border-border flex items-start gap-4 border-b p-5">
                <div className="surface-subtle text-text flex h-10 w-10 shrink-0 items-center justify-center rounded-md">
                  <Globe2 className="h-5 w-5" aria-hidden="true" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <h3 className="text-text font-semibold">{service.label}</h3>
                    <ConnectionStatus connection={connection} />
                  </div>
                  <p className="text-muted mt-1 text-sm">{service.connection?.description}</p>
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
                {refreshOAuth.error && refreshOAuth.variables === connection?.id ? (
                  <Alert tone="danger">
                    {t("requestFailed", { code: refreshOAuth.error.message })}
                  </Alert>
                ) : null}
              </div>

              <div className="border-border flex flex-wrap justify-end gap-2 border-t p-4">
                {connection && service.connection?.testable ? (
                  <Button
                    size="sm"
                    onClick={() => testConnection.mutate(connection.id)}
                    loading={testConnection.isPending && testConnection.variables === connection.id}
                  >
                    <Send className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("test")}
                  </Button>
                ) : null}
                {connection?.oauth_refresh_configured && service.connection?.oauth ? (
                  <Button
                    size="sm"
                    onClick={() => refreshOAuth.mutate(connection.id)}
                    loading={refreshOAuth.isPending && refreshOAuth.variables === connection.id}
                  >
                    {t("refreshOAuthToken")}
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

      {editing
        ? services
            .filter((service) => service.name === editing)
            .map((service) => (
              <ConnectionForm
                key={service.name}
                teamId={teamId}
                service={service}
                connection={connections.find((item) => item.service === service.name)}
                onClose={() => setEditing(null)}
              />
            ))
        : null}
      <ConfirmDialog
        open={!!deleting}
        title={t("disconnectTitle", { service: deleting?.service ?? "" })}
        description={t("disconnectDescription")}
        confirmLabel={t("disconnect")}
        cancelLabel={t("cancel")}
        intent="destructive"
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
