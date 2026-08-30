# Demo dataset and integration runs

This guide prepares the same deterministic, single-Team narrative locally or
in production. The SQL fixture contains fictional presentation content and is
safe to version. Real identities, cluster paths, tokens, webhook secrets, SMTP
credentials and outbound endpoints belong only in `.env`, which Git ignores.

The three-Team browser-test fixture remains separate in `tooling/seed_demo.*`.
Do not replace it with the presentation fixture: Playwright relies on its fixed
Teams and accounts.

## 1. Configure `.env`

Copy `.env.example` to `.env` once, then fill the applicable `DEMO_` fields.
There is no `.presentation.env`; the CLI reads only `.env` and process-level
environment variables.

### Target and identity

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_LOCAL_API_ORIGIN` | Local | Local Axum API, normally `http://localhost:8080`. Public origins are rejected for a local target. |
| `DEMO_PRODUCTION_API_ORIGIN` | Production | Public HTTPS API, normally `https://api.opswarden.dev`. Local or HTTP origins are rejected. |
| `DEMO_LOCAL_MANAGER_EMAIL` | Local | Manager of the local Team. Overrides `DEMO_MANAGER_EMAIL`. |
| `DEMO_PRODUCTION_MANAGER_EMAIL` | Production | Real verified OAuth email of the production Manager. Overrides `DEMO_MANAGER_EMAIL`. |
| `DEMO_MANAGER_EMAIL` | Fallback | Shared Manager email if no target-specific value is present. |
| `DEMO_LOCAL_MANAGER_TOKEN` | Optional | Short-lived local bearer token. Usually unnecessary because local password sign-in is available. |
| `DEMO_PRODUCTION_MANAGER_TOKEN` | Production integrations | Bearer token used to configure connections and rules for an OAuth Manager. Keep it private and refresh it when expired. `--prompt-token` avoids storing it. |
| `DEMO_RESPONDER_EMAIL` | Both | Dedicated fictional Responder account created by `bootstrap` or `seed`. |
| `DEMO_OBSERVER_EMAIL` | Both | Dedicated fictional Observer account created by `bootstrap` or `seed`. |
| `DEMO_CONTRACTOR_EMAIL` | Both | Dedicated fictional account used for the active-ban example. |
| `DEMO_PASSWORD` | Both | Presentation-only password for the dedicated accounts; never use a personal password. |
| `DEMO_LOCAL_PASSWORD` | Optional | Local override, useful for an existing E2E Manager account. |
| `DEMO_TEAM_NAME` | Fallback | Shared Team name when no target-specific name is set. |
| `DEMO_LOCAL_TEAM_NAME` | Local | Exact local Team name created through onboarding. |
| `DEMO_PRODUCTION_TEAM_NAME` | Production | Exact production Team name created through onboarding. |
| `DEMO_LOCAL_TEAM_ID` | Optional | Removes ambiguity if the local Manager owns several Teams with the same name. |
| `DEMO_PRODUCTION_TEAM_ID` | Recommended | Pins destructive production operations to the intended Team UUID. |

### Provider narrative and authentication

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_GITHUB_REPOSITORY` | Both | Exact `owner/repository` matched by the GitHub failure rule. |
| `DEMO_GITHUB_BRANCH` | Both | GitHub branch matched by the rule. |
| `DEMO_GITHUB_WORKFLOW` | Both | Exact GitHub Actions workflow name. |
| `DEMO_GITLAB_PROJECT` | Both | Exact GitLab `namespace/project` matched by the rule. |
| `DEMO_GITLAB_BRANCH` | Both | GitLab branch matched by the rule. |
| `DEMO_GITLAB_PIPELINE` | Both | Pipeline name used by the normalized event. |
| `DEMO_GITHUB_WEBHOOK_SECRET` | Both | Independent HMAC secret configured in OpsWarden and GitHub. This is not an OAuth secret. |
| `DEMO_GITLAB_WEBHOOK_SECRET` | Both | Token configured in OpsWarden and GitLab's webhook settings. |
| `DEMO_GENERIC_WEBHOOK_SECRET` | Both | Shared token carried in `X-OpsWarden-Token`. |
| `DEMO_ALERTMANAGER_WEBHOOK_SECRET` | Both | Bearer token used by Alertmanager. |

Generate four different values with `openssl rand -hex 32`. Never reuse Google
or GitHub OAuth client secrets for webhook authentication.

### Outbound reactions

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_HTTP_ENDPOINT` | Optional | Public HTTPS destination for the HTTP reaction, such as a fresh Webhook.site request bin. Local/private destinations are rejected. |
| `DEMO_SMTP_HOST` | Optional group | SMTP hostname. Setting any SMTP transport field requires the complete group. |
| `DEMO_SMTP_PORT` | Optional group | SMTP port, normally `587`. |
| `DEMO_SMTP_USERNAME` | Optional group | Dedicated SMTP username. |
| `DEMO_SMTP_PASSWORD` | Optional group | Dedicated SMTP password or provider app password. |
| `DEMO_EMAIL_FROM` | Optional group | Verified sender address. |
| `DEMO_EMAIL_TO` | Required with SMTP | Recipient used by the deterministic email rule. |

### Database target

| Variable | Required | Purpose |
| --- | --- | --- |
| `DEMO_KUBECONFIG` | Production | Absolute path to the production cluster kubeconfig. |
| `DEMO_KUBE_CONTEXT` | Production | Exact kubectl context. |
| `DEMO_KUBE_NAMESPACE` | Production | Namespace containing `deployment/postgres`. |
| `DEMO_DB_NAME` | Both | PostgreSQL database name. |
| `DEMO_LOCAL_DB_USER` | Local | PostgreSQL user inside the Compose `db` service. |
| `DEMO_PRODUCTION_DB_USER` | Production | PostgreSQL user used through `kubectl exec`. |

## 2. Prepare and run locally

Create the Team through the real onboarding flow before seeding it.

```bash
python3 tooling/demo.py bootstrap --target local
python3 tooling/demo.py doctor --target local
python3 tooling/demo.py seed --target local
python3 tooling/demo.py run --target local
python3 tooling/demo.py deseed --target local
```

`seed` is intentionally idempotent. It replaces incidents, releases,
automation rules, bans and non-Manager memberships in the selected Team; it
preserves the Team, Manager, user accounts and service connections. Use
`--data-only` only when connections and rules must not be configured.

## 3. Prepare and run in production

First create the production Manager and Team through real OAuth and onboarding.
The one-time bootstrap creates only the three dedicated role accounts.

```bash
python3 tooling/demo.py bootstrap --target production --confirm BOOTSTRAP_PRODUCTION
python3 tooling/demo.py doctor --target production
python3 tooling/demo.py seed --target production --confirm SEED_PRODUCTION
python3 tooling/demo.py run --target production
python3 tooling/demo.py deseed --target production --confirm DESEED_PRODUCTION
```

If `DEMO_PRODUCTION_MANAGER_TOKEN` is empty, add `--prompt-token` to `seed` or
run `integrations --target production --prompt-token` after a data-only seed.
Production seed and deseed require exact operation-specific confirmations. The
`run` command is not destructive, but it deliberately creates automation runs,
incidents and optional outbound HTTP/email notifications.

## 4. What each command owns

| Command | Effect |
| --- | --- |
| `doctor` | Read-only health, Team and identity validation. |
| `bootstrap` | Ensures dedicated accounts exist; never creates the production Manager or Team. |
| `seed` | Replaces the selected Team's narrative, then configures connections and enabled rules. |
| `integrations` | Reconfigures connections and upserts rules without replacing narrative data; standalone production use requires `--confirm INTEGRATIONS_PRODUCTION`. |
| `run` | Sends four authenticated inbound samples and reports persisted run statuses. |
| `deseed` | Removes deterministic narrative, rules and role memberships while preserving Team, users and connections. |

After a successful run, expect five runs: four incident-creation reactions and
one HTTP reaction when `DEMO_HTTP_ENDPOINT` is configured. SMTP adds a sixth.
