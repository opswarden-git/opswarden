# OpsWarden local demo data

## Rebuild the dataset

```bash
nix develop
just demo
```

This rebuilds the Compose stack, refreshes the deterministic demo rows and
records one signed GitHub webhook Run. It does not wipe unrelated local data.
Use `just demo-seed` when the stack is already running and no simulated Run is
needed.

| Account                     | Password | Manager of        | Other roles          |
| --------------------------- | -------- | ----------------- | -------------------- |
| `manager@opswarden.local`   | `sudo`   | OpsWarden Demo    | Responder / Observer |
| `responder@opswarden.local` | `sudo`   | Production Europe | Responder            |
| `observer@opswarden.local`  | `sudo`   | Security Lab      | Observer             |

The dataset covers every Incident status and severity, empty and assigned
states, edited timeline notes, reactions, private messages, bans, and Releases
in `created`, `in_progress`, derived `blocked`, `completed`, and `cancelled`
states.

## GitHub webhook

The seed connects GitHub to **OpsWarden Demo** through its Team-owned encrypted
connection and enables one durable GitHub CI failure → VIGIL Incident rule.
The local signing secret is:

```text
opswarden-demo-webhook-secret
```

Run a signed local failure delivery with:

```bash
just demo-webhook
```

The script discovers the Team's opaque `connectionId`; no global webhook exists.
Each unique delivery creates a high-severity Incident and a durable Run.

For GitHub.com, `localhost` is not reachable from GitHub's servers. Expose port
8080 through an HTTPS tunnel or deploy the server, then configure the repository:

1. **Settings → Webhooks → Add webhook**.
2. Copy the exact Payload URL from **Team → Automations → Connections**. It has
   the form `https://YOUR-PUBLIC-HOST/webhooks/github/:connectionId`.
3. Content type: `application/json`.
4. Secret: `opswarden-demo-webhook-secret`.
5. Select individual events, then enable **Workflow runs** only.
6. Keep the webhook active.

GitHub sends a `ping` when the hook is created. OpsWarden acknowledges it but
does not create an Incident; an Incident is created only when a `workflow_run`
payload has conclusion `failure`, `timed_out`, or `startup_failure`.

Official references:

- [Creating webhooks](https://docs.github.com/en/webhooks/using-webhooks/creating-webhooks)
- [The workflow_run webhook payload](https://docs.github.com/en/webhooks/webhook-events-and-payloads#workflow_run)
