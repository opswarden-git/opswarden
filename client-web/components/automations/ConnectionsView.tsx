"use client";

import { CheckCircle2, Clock3, Globe2, Unplug, Webhook } from "lucide-react";
import Image from "next/image";
import { useLocale, useTranslations } from "next-intl";
import React, { useState, useSyncExternalStore } from "react";
import { MdAlternateEmail, MdHttp } from "react-icons/md";
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
import { FormField } from "@/components/ui/FormField";
import { StatusBadge } from "@/components/ui/StatusBadge";

const providerMarks: Record<string, string> = {
  alertmanager: "/assets/alertmanager.svg",
  github: "/assets/github-patched.webp",
  gitlab: "/assets/gitlab.webp",
};

function ServiceMark({ service }: { service: AutomationService }) {
  const asset = providerMarks[service.name];
  if (asset) {
    return <Image src={asset} alt="" width={28} height={28} className="h-7 w-7 object-contain" />;
  }
  if (service.name === "email") {
    return <MdAlternateEmail className="h-7 w-7" aria-hidden="true" />;
  }
  if (service.name === "http") return <MdHttp className="h-8 w-8" aria-hidden="true" />;
  if (service.name === "generic") return <Webhook className="h-7 w-7" aria-hidden="true" />;
  return <Globe2 className="h-7 w-7" aria-hidden="true" />;
}

function ConnectionStatus({ connection }: { connection: TeamConnection }) {
  const t = useTranslations("Automations");
  if (connection.last_error_code) {
    return (
      <StatusBadge tone="danger" icon={<Unplug />}>
        {t("needsAttention")}
      </StatusBadge>
    );
  }
  if (connection.verified_at || connection.last_delivery_at) {
    return (
      <StatusBadge tone="success" icon={<CheckCircle2 />}>
        {t("verified")}
      </StatusBadge>
    );
  }
  return (
    <StatusBadge tone="warning" icon={<Clock3 />}>
      {t("awaitingVerification")}
    </StatusBadge>
  );
}

function ConnectionForm({
  connection,
  id,
  onClose,
  service,
  teamId,
}: {
  connection?: TeamConnection;
  id: string;
  onClose: () => void;
  service: AutomationService;
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const locale = useLocale();
  const fields = service.connection?.fields ?? [];
  const [values, setValues] = useState(() => catalogValues(fields));
  const configure = useConfigureTeamConnection(teamId);
  const startOAuth = useStartServiceOAuth(teamId);
  const valid = catalogFieldsAreValid(fields, values, !!connection);

  return (
    <form
      id={id}
      aria-label={
        connection
          ? t("reconfigureService", { service: service.label })
          : t("connectService", { service: service.label })
      }
      className="border-border bg-panel-2/20 border-t px-4 py-5 md:px-5"
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
      <div className="max-w-2xl space-y-5">
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
        {fields.map((field) => (
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
        <div className="flex justify-end gap-2 pt-1">
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
      </div>
    </form>
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
  const integrations = services.map((service) => ({
    service,
    connection: connections.find((item) => item.service === service.name),
  }));
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
      <div className="space-y-8">
        {[
          {
            id: "active-integrations",
            label: t("activeIntegrations"),
            emptyLabel: t("noActiveIntegrations"),
            items: integrations.filter(({ connection }) => connection),
          },
          {
            id: "inactive-integrations",
            label: t("inactiveIntegrations"),
            emptyLabel: t("noInactiveIntegrations"),
            items: integrations.filter(({ connection }) => !connection),
          },
        ].map((group) => (
          <section key={group.id} aria-labelledby={group.id}>
            <div className="mb-2 flex items-baseline gap-2 px-1">
              <h2 id={group.id} className="text-text text-sm font-semibold">
                {group.label}
              </h2>
              <span className="text-muted text-xs tabular-nums">{group.items.length}</span>
            </div>
            <div className="surface divide-border divide-y overflow-hidden rounded-md">
              {group.items.length === 0 ? (
                <p className="text-muted px-4 py-4 text-sm">{group.emptyLabel}</p>
              ) : (
                group.items.map(({ connection, service }) => {
                  const panelId = `integration-panel-${service.name}`;
                  const isExpanded = editing === service.name;
                  const usedBy = connection
                    ? rules.filter(
                        (rule) =>
                          rule.trigger_connection_id === connection.id ||
                          rule.reaction_connection_id === connection.id,
                      ).length
                    : 0;

                  return (
                    <div key={service.name}>
                      <div className="flex min-h-16 items-center gap-4 px-4 py-3">
                        <div className="text-text flex h-8 w-8 shrink-0 items-center justify-center">
                          <ServiceMark service={service} />
                        </div>
                        <h3 className="text-text min-w-0 flex-1 truncate text-sm font-medium">
                          {service.label}
                        </h3>
                        {connection ? <ConnectionStatus connection={connection} /> : null}
                        <Button
                          size="sm"
                          variant={connection ? "secondary" : "primary"}
                          aria-expanded={isExpanded}
                          aria-controls={panelId}
                          onClick={() => setEditing(isExpanded ? null : service.name)}
                        >
                          {connection ? t("configure") : t("connect")}
                        </Button>
                      </div>

                      {isExpanded ? (
                        <div id={panelId}>
                          {connection ? (
                            <div className="border-border space-y-4 border-t px-4 py-4 md:px-5">
                              {connection.webhook_path ? (
                                <div className="flex items-center gap-2">
                                  <code className="text-muted min-w-0 flex-1 truncate text-xs">
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
                              ) : null}
                              <div className="flex flex-wrap items-end justify-between gap-4">
                                <dl className="flex flex-wrap gap-x-8 gap-y-2 text-xs">
                                  <div>
                                    <dt className="text-muted">{t("lastActivity")}</dt>
                                    <dd className="text-text mt-1">
                                      {connection.last_delivery_at || connection.verified_at
                                        ? new Intl.DateTimeFormat(locale, {
                                            dateStyle: "medium",
                                            timeStyle: "short",
                                          }).format(
                                            new Date(
                                              connection.last_delivery_at ??
                                                connection.verified_at!,
                                            ),
                                          )
                                        : t("never")}
                                    </dd>
                                  </div>
                                  <div>
                                    <dt className="text-muted">{t("usedByRules")}</dt>
                                    <dd className="text-text mt-1 tabular-nums">{usedBy}</dd>
                                  </div>
                                </dl>
                                <div className="flex flex-wrap gap-2">
                                  {service.connection?.testable ? (
                                    <Button
                                      size="sm"
                                      onClick={() => testConnection.mutate(connection.id)}
                                      loading={
                                        testConnection.isPending &&
                                        testConnection.variables === connection.id
                                      }
                                    >
                                      {t("test")}
                                    </Button>
                                  ) : null}
                                  {connection.oauth_refresh_configured &&
                                  service.connection?.oauth ? (
                                    <Button
                                      size="sm"
                                      onClick={() => refreshOAuth.mutate(connection.id)}
                                      loading={
                                        refreshOAuth.isPending &&
                                        refreshOAuth.variables === connection.id
                                      }
                                    >
                                      {t("refreshOAuthToken")}
                                    </Button>
                                  ) : null}
                                  <Button
                                    size="sm"
                                    onClick={() => setDeleting(connection)}
                                    disabled={usedBy > 0}
                                    title={
                                      usedBy > 0
                                        ? t("connectionInUse", { count: usedBy })
                                        : undefined
                                    }
                                  >
                                    {t("disconnect")}
                                  </Button>
                                </div>
                              </div>
                              {connection.last_error_code ? (
                                <Alert tone="danger">
                                  {t("lastError", { code: connection.last_error_code })}
                                </Alert>
                              ) : null}
                              {testConnection.error &&
                              testConnection.variables === connection.id ? (
                                <Alert tone="danger">
                                  {t("requestFailed", { code: testConnection.error.message })}
                                </Alert>
                              ) : null}
                              {testConnection.isSuccess &&
                              testConnection.variables === connection.id ? (
                                <Alert tone="success">{t("testSucceeded")}</Alert>
                              ) : null}
                              {refreshOAuth.error && refreshOAuth.variables === connection.id ? (
                                <Alert tone="danger">
                                  {t("requestFailed", { code: refreshOAuth.error.message })}
                                </Alert>
                              ) : null}
                            </div>
                          ) : null}
                          <ConnectionForm
                            id={`${panelId}-form`}
                            teamId={teamId}
                            service={service}
                            connection={connection}
                            onClose={() => setEditing(null)}
                          />
                        </div>
                      ) : null}
                    </div>
                  );
                })
              )}
            </div>
          </section>
        ))}
      </div>
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
