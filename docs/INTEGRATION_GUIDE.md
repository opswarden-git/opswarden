# Integration and demo guide

This is the canonical guide for OpsWarden identity OAuth, Team integrations,
and the reproducible single-Team demonstration dataset.

## Identity OAuth versus Team integrations

Google and GitHub identity OAuth sign a user up or in. GitHub Team OAuth is a
separate authorization: it grants one Team's automation connection access to
GitHub and stores encrypted access and refresh tokens. Webhook secrets are
separate again and authenticate inbound provider deliveries.

The versioned Python and SQL fixtures contain only fictional content. Real
emails, Team IDs, cluster paths, bearer tokens, webhook secrets, SMTP
credentials, and outbound endpoints belong only in the Git-ignored `.env`.
There is no `.presentation.env` and no `tooling/presentation/`.

The historical three-Team browser fixture remains in `tooling/seed_demo.*` for
Playwright. The presentation engine is `tooling/demo.py` plus `tooling/demo/`.

## Configure `.env`

Copy `.env.example` to `.env`, then fill the applicable `DEMO_` fields.
Process-level variables override values loaded from `.env`.

### Target and identity

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_LOCAL_API_ORIGIN` | Local | Local API, normally `http://localhost:8080`; public origins are rejected. |
| `DEMO_PRODUCTION_API_ORIGIN` | Production | Public HTTPS API, normally `https://api.opswarden.dev`; local or HTTP origins are rejected. |
| `DEMO_LOCAL_MANAGER_EMAIL` | Local | Manager of the local Team; overrides `DEMO_MANAGER_EMAIL`. |
| `DEMO_PRODUCTION_MANAGER_EMAIL` | Production | Verified OAuth email of the production Manager; overrides `DEMO_MANAGER_EMAIL`. |
| `DEMO_MANAGER_EMAIL` | Fallback | Shared Manager email when no target-specific value exists. |
| `DEMO_LOCAL_MANAGER_TOKEN` | Optional | Short-lived local bearer token; password sign-in is normally simpler locally. |
| `DEMO_PRODUCTION_MANAGER_TOKEN` | Production integrations | Short-lived bearer token for configuring Team connections and rules. Keep private; `--prompt-token` avoids storing it. |
| `DEMO_RESPONDER_EMAIL` | Both | Dedicated fictional Responder created by `bootstrap` or `seed`. |
| `DEMO_OBSERVER_EMAIL` | Both | Dedicated fictional Observer created by `bootstrap` or `seed`. |
| `DEMO_CONTRACTOR_EMAIL` | Both | Dedicated fictional account used for the active-ban example. |
| `DEMO_PASSWORD` | Both | Presentation-only password for dedicated accounts; never use a personal password. |
| `DEMO_LOCAL_PASSWORD` | Optional | Local override for an existing E2E Manager account. |
| `DEMO_TEAM_NAME` | Fallback | Shared Team name when no target-specific name exists. |
| `DEMO_LOCAL_TEAM_NAME` | Local | Exact local Team name created through onboarding. |
| `DEMO_PRODUCTION_TEAM_NAME` | Production | Exact production Team name created through onboarding. |
| `DEMO_LOCAL_TEAM_ID` | Recommended | Pins local replacement to the intended Team UUID. |
| `DEMO_PRODUCTION_TEAM_ID` | Required operationally | Pins destructive production operations to the intended Team UUID. |

### Provider narrative and authentication

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_GITHUB_REPOSITORY` | Both | Exact GitHub `owner/repository` matched by the CI rule. |
| `DEMO_GITHUB_BRANCH` | Both | GitHub branch matched by the rule. |
| `DEMO_GITHUB_WORKFLOW` | Both | Exact GitHub Actions workflow name. |
| `DEMO_GITLAB_PROJECT` | Both | Exact GitLab `namespace/project` matched by the CI rule. |
| `DEMO_GITLAB_BRANCH` | Both | GitLab branch matched by the rule. |
| `DEMO_GITLAB_PIPELINE` | Both | Pipeline name used by the normalized event. |
| `DEMO_GITHUB_WEBHOOK_SECRET` | Both | Independent HMAC secret configured in OpsWarden and GitHub; not an OAuth secret. |
| `DEMO_GITLAB_WEBHOOK_SECRET` | Both | Token configured in OpsWarden and GitLab webhook settings. |
| `DEMO_GENERIC_WEBHOOK_SECRET` | Both | Shared token carried in `X-OpsWarden-Token`. |
| `DEMO_ALERTMANAGER_WEBHOOK_SECRET` | Both | Bearer token sent by Alertmanager. |

Generate four different values with `openssl rand -hex 32`. Never reuse Google
or GitHub OAuth client secrets for webhook authentication.

### Outbound reactions

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_HTTP_ENDPOINT` | Optional | Public HTTPS HTTP-reaction destination, such as a fresh Webhook.site bin. |
| `DEMO_SMTP_HOST` | Optional group | SMTP hostname; setting one transport field requires the complete group. |
| `DEMO_SMTP_PORT` | Optional group | SMTP port, normally `587`. |
| `DEMO_SMTP_USERNAME` | Optional group | Dedicated SMTP username. |
| `DEMO_SMTP_PASSWORD` | Optional group | Dedicated SMTP password or app password. |
| `DEMO_EMAIL_FROM` | Optional group | Provider-verified sender address. |
| `DEMO_EMAIL_TO` | With SMTP | Recipient used by the deterministic email rule. |

### Database target

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_KUBECONFIG` | Production | Absolute path to the production kubeconfig. |
| `DEMO_KUBE_CONTEXT` | Production | Exact kubectl context. |
| `DEMO_KUBE_NAMESPACE` | Production | Namespace containing `deployment/postgres`. |
| `DEMO_DB_NAME` | Both | PostgreSQL database name. |
| `DEMO_LOCAL_DB_USER` | Local | PostgreSQL user in the Compose `db` service. |
| `DEMO_PRODUCTION_DB_USER` | Production | PostgreSQL user used through `kubectl exec`. |

## Run the fixture locally

Create the Team through the real onboarding flow first.

```bash
python3 tooling/demo.py bootstrap --target local
python3 tooling/demo.py doctor --target local
python3 tooling/demo.py seed --target local
python3 tooling/demo.py run --target local
python3 tooling/demo.py deseed --target local
```

`seed` is idempotent. It replaces incidents, releases, automation rules, bans,
and non-Manager memberships in the selected Team while preserving the Team,
Manager, user accounts, and service connections. `--data-only` skips connection
and rule configuration.

## Run the fixture in production

Create the production Manager and Team through real OAuth and onboarding. The
one-time bootstrap creates only the three dedicated role accounts.

```bash
python3 tooling/demo.py bootstrap --target production --confirm BOOTSTRAP_PRODUCTION
python3 tooling/demo.py doctor --target production
python3 tooling/demo.py seed --target production --confirm SEED_PRODUCTION
python3 tooling/demo.py run --target production
python3 tooling/demo.py deseed --target production --confirm DESEED_PRODUCTION
```

If `DEMO_PRODUCTION_MANAGER_TOKEN` is empty, add `--prompt-token` to `seed`.
Standalone production integration configuration requires
`--confirm INTEGRATIONS_PRODUCTION`. `run` deliberately creates automation
runs, incidents, and optional outbound HTTP/email notifications.

| Command | Effect |
| --- | --- |
| `doctor` | Read-only health, Team, and identity validation. |
| `bootstrap` | Ensures dedicated accounts exist; never creates the production Manager or Team. |
| `seed` | Replaces the Team narrative, then configures connections and enabled rules. |
| `integrations` | Reconfigures connections and upserts rules without replacing narrative data. |
| `run` | Sends four authenticated inbound samples and reports persisted run statuses. |
| `deseed` | Removes deterministic narrative, rules, and role memberships while preserving Team, users, and connections. |

A complete run produces five successes: four incident reactions and one HTTP
reaction when `DEMO_HTTP_ENDPOINT` is configured. SMTP adds a sixth.

## Alertmanager contract

OpsWarden evaluates every authenticated Alertmanager alert as an independent
`alert_firing` or `alert_resolved` lifecycle transition.

### Configure the connection

1. Open the Team's Automations page as Manager.
2. Configure Alertmanager with a dedicated high-entropy bearer token.
3. Copy the returned `/webhooks/alertmanager/<connection-id>` path.
4. Create and enable the required firing/resolved rules.
5. Configure Alertmanager without placing the token in its main YAML:

```yaml
route:
  receiver: opswarden

receivers:
  - name: opswarden
    webhook_configs:
      - url: https://api.opswarden.dev/webhooks/alertmanager/CONNECTION_ID
        send_resolved: true
        http_config:
          authorization:
            type: Bearer
            credentials_file: /run/secrets/opswarden_alertmanager_token
```

The mounted file contains only the token configured in OpsWarden. Validate and
reload using the [official Alertmanager configuration procedure](https://prometheus.io/docs/alerting/latest/configuration/#http_config).

### Behavior, filters, and templates

| Input | Result |
| --- | --- |
| `status: firing` | Evaluated against enabled `alert_firing` rules. |
| `status: resolved` with `endsAt` | Evaluated against enabled `alert_resolved` rules. |
| Mixed group | Split into one transition per alert. |
| Semantic retry | Accepted as duplicate without another run. |
| No matching rule | Durably ignored and counted as ignored. |
| Invalid bearer, schema, JSON, or body above 1 MiB | Rejected before reserving a delivery. |
| Reaction error | Persisted as a failed run and metric. |

Semantic identity includes `groupKey`, receiver, fingerprint, status, and
`startsAt`; resolved events also include `endsAt`. Formatting, annotations,
labels, and a firing alert's changing `endsAt` do not alter identity.

Rules can filter on `severity`, `alertname`, `receiver`, `instance`,
`namespace`, `pod`, `service`, `job`, and `status`. Templates also expose:

```text
{{summary}} {{description}} {{group_key}} {{fingerprint}}
{{starts_at}} {{ends_at}} {{generator_url}}
```

Unknown keys, nested values, credentials, and raw payloads are never template
variables. Normalized values are non-empty and bounded to 1,024 bytes.

### Metrics, rotation, and troubleshooting

`GET /metrics` exposes
`opswarden_alertmanager_webhook_deliveries_total{outcome="..."}` for
`accepted`, `rejected`, `duplicate`, `ignored`, and `failed`.

Rotate by updating OpsWarden, updating the mounted secret, reloading
Alertmanager, sending a test, and then removing the old token. Only one token is
accepted per connection, so coordinate the change in a short window.

| Symptom | Check |
| --- | --- |
| `400 Bad Request` | Content type, bearer shape, size limit, and lifecycle fields. |
| `401 Unauthorized` | Token matches the current Team connection. |
| `404` connection error | ID belongs to an Alertmanager connection. |
| `202` duplicate | The semantic transition was already processed. |
| `202` ignored | Rule disabled, filters differ, or no matching lifecycle rule. |
| Failed run | Reaction connection health and persisted automation error. |

Never log or paste the bearer token while troubleshooting.
