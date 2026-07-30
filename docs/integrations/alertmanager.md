# Alertmanager integration

OpsWarden `v1.0.11` can receive authenticated Alertmanager notification groups
and trigger team automation rules from firing groups.

## Configure OpsWarden

1. Open the team's **Automations** page as a Manager.
2. Create or update the **Alertmanager** connection.
3. Generate a dedicated high-entropy token and store it in a secret manager.
4. Enter the token once. OpsWarden encrypts it and returns only the connection
   metadata and webhook path.
5. Copy the path:

   ```text
   /webhooks/alertmanager/<connection-id>
   ```

6. Create an `alert_firing` rule and enable it. Optional filters are
   `severity`, `alertname` and `receiver`.

Use HTTPS for the public URL. Do not reuse an administrator password, API token
or another integration's secret.

## Configure Alertmanager

The official Alertmanager webhook configuration supports an `http_config`
authorization block. Prefer a mounted credentials file so the token is not
embedded in the main configuration:

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

The `credentials_file` must contain only the same token configured in OpsWarden.
The syntax follows the
[official Prometheus Alertmanager configuration](https://prometheus.io/docs/alerting/latest/configuration/#http_config).

Validate and reload Alertmanager using the procedure appropriate for your
deployment. Keep the old configuration available for rollback.

## Supported behavior

| Input                                                    | Result                                                                            |
| -------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Top-level `status: firing` with at least one valid alert | Accepted and evaluated against enabled `alert_firing` rules                       |
| Top-level `status: resolved`                             | Accepted, deduplicated and durably marked ignored; no rule or incident transition |
| Exact retry of the same raw body                         | Accepted as a duplicate without another automation run                            |
| Changed raw body                                         | Treated as a new delivery                                                         |
| Missing or invalid bearer token                          | Rejected before reserving a delivery                                              |
| Non-JSON content type, malformed JSON or invalid schema  | Rejected                                                                          |
| Body above 1 MiB                                         | Rejected by the HTTP body limit                                                   |

One request represents one Alertmanager notification group. A firing group
creates at most one external event, even when it contains several alerts.

## Normalized data

Rules can filter on:

- `severity`, when shared by every alert;
- `alertname`, when shared by every alert;
- `receiver`.

Reaction templates may use these scalar variables when present:

```text
{{severity}}
{{alertname}}
{{summary}}
{{receiver}}
{{group_key}}
```

The normalized per-alert projection retains only status, fingerprint, alert
name, severity and summary. Arbitrary labels, annotations, bearer tokens and the
raw payload are never exposed as template variables.

## Token rotation

1. Generate a new high-entropy token.
2. Update the Alertmanager connection in OpsWarden with the new token.
3. Update the mounted Alertmanager secret.
4. Reload Alertmanager.
5. Send a test firing group and verify an accepted, non-duplicate delivery.
6. Remove the old token from the secret store.

OpsWarden accepts one token per connection, so coordinate steps 2–4 in a short
maintenance window.

## Troubleshooting

| Symptom                                  | Check                                                                                    |
| ---------------------------------------- | ---------------------------------------------------------------------------------------- |
| `400 Bad Request`                        | JSON content type, Bearer header shape, payload schema and non-empty firing alerts       |
| `401 Unauthorized`                       | The Alertmanager token exactly matches the current team connection token                 |
| `404`-style connection error             | The connection ID belongs to an existing Alertmanager connection                         |
| `202 Accepted` with `duplicate: true`    | The exact raw body was already processed                                                 |
| `202 Accepted` with zero triggered rules | Top-level status may be resolved, the rule may be disabled, or its filters may not match |
| Rule failure                             | Inspect the team's automation runs and the reaction connection health                    |

Do not log or paste the bearer token while troubleshooting. A successful
authenticated delivery resets the connection health; a bad token does not
reserve a delivery.
