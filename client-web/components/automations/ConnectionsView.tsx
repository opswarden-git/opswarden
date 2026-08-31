"use client";

import { CheckCircle2, ChevronRight, Clock3, Globe2, Unplug, Webhook } from "lucide-react";
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
import { useErrorText } from "@/lib/useErrorText";

const providerMarks: Record<string, string> = {
  alertmanager: "/assets/alertmanager.svg",
  github: "/assets/github-patched.webp",
  gitlab: "/assets/gitlab.webp",
};

/**
 * The mark of a connector: its brand asset when we ship one, a shape otherwise.
 * `inline` is the smaller size used on the button that signs into the service,
 * where the label sits next to it. Decorative in both cases — the surrounding
 * row or button already names the service — and the one place that decides what
 * a service looks like.
 */
function ServiceMark({ inline, service }: { inline?: boolean; service: AutomationService }) {
  const box = inline ? "h-[18px] w-[18px]" : "h-7 w-7";
  const asset = providerMarks[service.name];
  if (asset) {
    const side = inline ? 18 : 28;
    return (
      <Image src={asset} alt="" width={side} height={side} className={`${box} object-contain`} />
    );
  }
  if (service.name === "email") {
    return <MdAlternateEmail className={box} aria-hidden="true" />;
  }
  if (service.name === "http")
    return <MdHttp className={inline ? box : "h-8 w-8"} aria-hidden="true" />;
  if (service.name === "generic") return <Webhook className={box} aria-hidden="true" />;
  return <Globe2 className={box} aria-hidden="true" />;
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
  if (!connection.webhook_path) {
    return (
      <StatusBadge tone="warning" icon={<Clock3 />}>
        {t("readyToTest")}
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
  const errorText = useErrorText();
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
      className="px-4 py-4"
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
      <div className="space-y-4">
        <div className="flex flex-wrap items-start gap-3">
          {fields.map((field) => (
            <FormField
              key={field.name}
              className="min-w-56 flex-1"
              label={field.label}
              caption={field.name === "endpoint_url" ? field.description : undefined}
              required={field.required}
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
          <div className={`flex items-center gap-2 ${fields.length > 0 ? "mt-7" : ""}`}>
            <Button
              type="submit"
              size="lg"
              variant="primary"
              disabled={!valid}
              loading={configure.isPending}
            >
              {t("connect")}
            </Button>
            {service.connection?.oauth ? (
              <Button
                type="button"
                size="lg"
                variant="secondary"
                className="border-0 bg-[#0d1117] text-white hover:bg-[#161b22]"
                loading={startOAuth.isPending}
                onClick={() =>
                  startOAuth.mutate(
                    { locale, service: service.name },
                    {
                      onSuccess: ({ authorization_url }) =>
                        window.location.assign(authorization_url),
                    },
                  )
                }
              >
                <ServiceMark inline service={service} />
                {service.connection.oauth.label}
              </Button>
            ) : null}
          </div>
        </div>
        {connection && fields.length > 0 ? (
          <p className="text-muted text-xs">{t("blankPreservesExisting")}</p>
        ) : null}
        {startOAuth.error ? (
          <Alert tone="danger">
            {t("requestFailed", { code: errorText(startOAuth.error.message) })}
          </Alert>
        ) : null}
        {configure.error ? (
          <Alert tone="danger">
            {t("requestFailed", { code: errorText(configure.error.message) })}
          </Alert>
        ) : null}
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
  const errorText = useErrorText();
  const locale = useLocale();
  const [editing, setEditing] = useState<string | null>(null);
  const [configuring, setConfiguring] = useState<string | null>(null);
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
            items: integrations.filter(({ connection }) => connection),
          },
          {
            id: "inactive-integrations",
            label: t("inactiveIntegrations"),
            items: integrations.filter(({ connection }) => !connection),
          },
        ]
          .filter((group) => group.items.length > 0)
          .map((group) => (
            <section key={group.id} aria-labelledby={group.id}>
              <div className="mb-2 flex items-baseline gap-2 px-1">
                <h2 id={group.id} className="text-text text-sm font-semibold">
                  {group.label}
                </h2>
                <span className="text-muted text-xs tabular-nums">{group.items.length}</span>
              </div>
              <div className="surface divide-border-muted divide-y overflow-hidden rounded-md">
                {group.items.map(({ connection, service }) => {
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
                        <button
                          type="button"
                          className="text-muted hover:text-text focus-visible:ring-gold/50 flex h-8 w-8 shrink-0 items-center justify-center rounded-sm transition-colors focus-visible:ring-2 focus-visible:outline-none"
                          aria-label={
                            connection
                              ? t("manageService", { service: service.label })
                              : t("connectService", { service: service.label })
                          }
                          aria-expanded={isExpanded}
                          aria-controls={panelId}
                          onClick={() => {
                            setEditing(isExpanded ? null : service.name);
                            setConfiguring(null);
                          }}
                        >
                          <ChevronRight
                            className={`h-4 w-4 transition-transform ${isExpanded ? "rotate-90" : ""}`}
                            aria-hidden="true"
                          />
                        </button>
                      </div>

                      {isExpanded ? (
                        <div id={panelId}>
                          {connection ? (
                            <div className="space-y-4 px-4 py-4">
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
                                  <Button
                                    size="sm"
                                    onClick={() =>
                                      setConfiguring((current) =>
                                        current === service.name ? null : service.name,
                                      )
                                    }
                                  >
                                    {t("reconfigure")}
                                  </Button>
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
                                  {t("lastError", {
                                    code: errorText(connection.last_error_code),
                                  })}
                                </Alert>
                              ) : null}
                              {testConnection.error &&
                              testConnection.variables === connection.id ? (
                                <Alert tone="danger">
                                  {t("requestFailed", {
                                    code: errorText(testConnection.error.message),
                                  })}
                                </Alert>
                              ) : null}
                              {testConnection.isSuccess &&
                              testConnection.variables === connection.id ? (
                                <Alert tone="success">{t("testSucceeded")}</Alert>
                              ) : null}
                              {refreshOAuth.error && refreshOAuth.variables === connection.id ? (
                                <Alert tone="danger">
                                  {t("requestFailed", {
                                    code: errorText(refreshOAuth.error.message),
                                  })}
                                </Alert>
                              ) : null}
                              {configuring === service.name ? (
                                <ConnectionForm
                                  id={`${panelId}-form`}
                                  teamId={teamId}
                                  service={service}
                                  connection={connection}
                                  onClose={() => setConfiguring(null)}
                                />
                              ) : null}
                            </div>
                          ) : null}
                          {!connection ? (
                            <ConnectionForm
                              id={`${panelId}-form`}
                              teamId={teamId}
                              service={service}
                              onClose={() => setEditing(null)}
                            />
                          ) : null}
                        </div>
                      ) : null}
                    </div>
                  );
                })}
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
            ? t("requestFailed", { code: errorText(deleteConnection.error.message) })
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
