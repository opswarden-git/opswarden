# Alertmanager integration

OpsWarden receives authenticated Alertmanager notification groups and evaluates
each alert as an independent firing or resolved lifecycle transition.

## Configure OpsWarden

1. Open the team's **Automations** page as a Manager.
2. Create or update the **Alertmanager** connection.
3. Generate a dedicated high-entropy token and store it in a secret manager.
4. Enter the token once. OpsWarden encrypts it and returns only connection
   metadata and the webhook path.
5. Create and enable `alert_firing` and, where useful, `alert_resolved` rules.

The endpoint is:

```text
https://opswarden.example.com/webhooks/alertmanager/<connection-id>
```

Do not reuse an administrator password, API token or another integration's
secret.

## Configure Alertmanager

Use Alertmanager's `http_config` authorization block. A mounted credentials
file keeps the token outside the main configuration:

```yaml
route:
  receiver: opswarden

receivers:
  - name: opswarden
    webhook_configs:
      - url: https://opswarden.example.com/webhooks/alertmanager/CONNECTION_ID
        send_resolved: true
        http_config:
          authorization:
            type: Bearer
            credentials_file: /run/secrets/opswarden_alertmanager_token
```

The file must contain only the token configured in OpsWarden. Validate and
reload the configuration using the
[official Alertmanager procedure](https://prometheus.io/docs/alerting/latest/configuration/#http_config).
Keep the previous configuration available for rollback.

## Supported behavior

| Input                                        | Result                                                        |
| -------------------------------------------- | ------------------------------------------------------------- |
| Alert with `status: firing`                  | Evaluated against enabled `alert_firing` rules                |
| Alert with `status: resolved` and `endsAt`   | Evaluated against enabled `alert_resolved` rules              |
| Mixed group                                  | Split into one transition per alert using each alert's status |
| Semantic retry                               | Accepted as duplicate without another run                     |
| No matching enabled rule                     | Accepted, durably ignored and counted as ignored              |
| Missing/invalid bearer token, schema or JSON | Rejected before a delivery is reserved                        |
| Body above 1 MiB                             | Rejected                                                      |
| Reaction error                               | Run persists as failed and the failed metric increments       |

The response summarizes `transitions_received`, `transitions_duplicate`,
`transitions_ignored`, `rules_triggered` and `rules_failed`. `duplicate` is true
only when every transition in the request was already known.

## Identity and mixed states

The semantic identity uses `groupKey`, receiver, fingerprint, per-alert status
and `startsAt`; a resolved transition also includes `endsAt`. Formatting,
annotations, labels and a firing alert's changing `endsAt` do not affect it.
A new alert start or a firing-to-resolved transition remains distinct.

See [ADR 0002](../adr/0002-alertmanager-lifecycle-contract.md) for the complete
decision.

## Filters and templates

Rules can filter on normalized scalar fields including `severity`, `alertname`,
`receiver`, `instance`, `namespace`, `pod`, `service`, `job` and `status`.
Templates can additionally use:

```text
{{summary}} {{description}} {{group_key}} {{fingerprint}}
{{starts_at}} {{ends_at}} {{generator_url}}
```

Every value is non-empty and at most 1,024 bytes. Unknown keys, nested values,
credentials and the raw payload are never exposed as template variables.

## Metrics

Scrape `GET /metrics`. The Alertmanager series is:

```text
opswarden_alertmanager_webhook_deliveries_total{outcome="accepted"}
opswarden_alertmanager_webhook_deliveries_total{outcome="rejected"}
opswarden_alertmanager_webhook_deliveries_total{outcome="duplicate"}
opswarden_alertmanager_webhook_deliveries_total{outcome="ignored"}
opswarden_alertmanager_webhook_deliveries_total{outcome="failed"}
```

## Token rotation

1. Generate a new high-entropy token.
2. Update the OpsWarden connection.
3. Update the mounted Alertmanager secret.
4. Reload Alertmanager.
5. Send a test alert and verify an accepted transition.
6. Remove the old token.

OpsWarden accepts one token per connection, so coordinate steps 2–4 in a short
maintenance window.

## Troubleshooting

| Symptom                   | Check                                                                 |
| ------------------------- | --------------------------------------------------------------------- |
| `400 Bad Request`         | Content type, bearer shape, 1 MiB limit and required lifecycle fields |
| `401 Unauthorized`        | Token matches the current team connection                             |
| `404` connection error    | ID belongs to an Alertmanager connection, not another provider        |
| `202` and duplicate count | Semantic transition was already processed                             |
| `202` and ignored count   | Rule is disabled, filters differ or no rule handles that status       |
| Failed outcome            | Team automation run and reaction connection health                    |

Never log or paste the bearer token while troubleshooting.
