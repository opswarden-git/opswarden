# Integrations

OpsWarden keeps integration credentials encrypted and scoped to one Team. An
incoming provider connection can trigger one or more Action → REAction rules;
outbound HTTP and Email connections deliver their effects without exposing
credentials to the browser or rule payloads.

## Provider guides

- [Alertmanager](alertmanager.md) — bearer authentication, lifecycle semantics,
  idempotency, metrics and token rotation.

GitHub, GitLab and Generic Webhook setup follows the fields and callback paths
shown in **Team → Integrations**. Provider-specific guides should be added here
as separate pages when their operational contract needs more explanation.
