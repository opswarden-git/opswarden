#!/usr/bin/env python3
"""Prepare, exercise and remove the single-Team OpsWarden demo fixture."""

from __future__ import annotations

import argparse
import getpass
import hashlib
import hmac
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen
from uuid import UUID, uuid4


ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "tooling" / "demo"
class DemoError(RuntimeError):
    pass


def parse_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.exists():
        return values
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key, value = key.strip(), value.strip()
        if value[:1] == value[-1:] and value[:1] in {'"', "'"}:
            value = value[1:-1]
        values[key] = value
    return values


def load_config() -> dict[str, str]:
    values = parse_env_file(ROOT / ".env")
    values.update(os.environ)
    return values


def value(config: dict[str, str], name: str, default: str = "") -> str:
    return config.get(name, default).strip()


DEFAULT_DEMO_VALUES = {
    "PASSWORD": "sudo",
    "MANAGER_EMAIL": "manager@opswarden.local",
    "RESPONDER_EMAIL": "responder@opswarden.local",
    "OBSERVER_EMAIL": "observer@opswarden.local",
    "CONTRACTOR_EMAIL": "contractor@opswarden.local",
    "TEAM_NAME": "OpsWarden",
}


def target_value(config: dict[str, str], target: str, name: str, default: str = "") -> str:
    fallback = default or DEFAULT_DEMO_VALUES.get(name, "")
    return value(config, f"DEMO_{target.upper()}_{name}") or value(config, f"DEMO_{name}", fallback)


def require(config: dict[str, str], *names: str) -> None:
    missing = [name for name in names if not value(config, name)]
    if missing:
        raise DemoError("Missing demo configuration: " + ", ".join(missing))


def api_origin(config: dict[str, str], target: str) -> str:
    specific = value(config, f"DEMO_{target.upper()}_API_ORIGIN")
    origin = specific or value(config, "DEMO_API_ORIGIN", "http://localhost:8080")
    parsed = urlparse(origin)
    is_local = parsed.hostname in {"localhost", "127.0.0.1"}
    if target == "local" and not is_local:
        raise DemoError(f"Local target refuses non-local API origin: {origin}")
    if target == "production" and (parsed.scheme != "https" or is_local):
        raise DemoError(f"Production target requires a public HTTPS API origin: {origin}")
    return origin.rstrip("/")


def require_confirmation(args: argparse.Namespace, operation: str) -> None:
    if args.target != "production":
        return
    expected = f"{operation.upper()}_PRODUCTION"
    if args.confirm != expected:
        raise DemoError(f"Production requires --confirm {expected}")


def request_json(
    origin: str,
    path: str,
    *,
    method: str = "GET",
    token: str = "",
    payload: object | None = None,
    headers: dict[str, str] | None = None,
) -> tuple[int, object | None]:
    body = None if payload is None else json.dumps(payload, separators=(",", ":")).encode()
    request_headers = dict(headers or {})
    if payload is not None:
        request_headers["Content-Type"] = "application/json"
    if token:
        request_headers["Authorization"] = f"Bearer {token}"
    request = Request(origin + path, data=body, headers=request_headers, method=method)
    try:
        with urlopen(request, timeout=20) as response:
            raw = response.read()
            return response.status, json.loads(raw) if raw else None
    except HTTPError as error:
        raw = error.read()
        try:
            parsed = json.loads(raw) if raw else None
        except json.JSONDecodeError:
            parsed = raw.decode(errors="replace")
        return error.code, parsed
    except URLError as error:
        raise DemoError(f"API request failed: {error.reason}") from error


class Database:
    def __init__(self, config: dict[str, str], target: str):
        self.target = target
        db_name = value(config, "DEMO_DB_NAME", "opswarden")
        if target == "local":
            user = value(config, "DEMO_LOCAL_DB_USER", "opswarden")
            self.command = ["docker", "compose", "exec", "-T", "db", "psql", "-X", "-U", user, "-d", db_name]
        else:
            require(config, "DEMO_KUBECONFIG", "DEMO_KUBE_CONTEXT")
            user = value(config, "DEMO_PRODUCTION_DB_USER", "postgres")
            namespace = value(config, "DEMO_KUBE_NAMESPACE", "default")
            self.command = [
                "kubectl", "--kubeconfig", value(config, "DEMO_KUBECONFIG"),
                "--context", value(config, "DEMO_KUBE_CONTEXT"), "-n", namespace,
                "exec", "-i", "deployment/postgres", "--", "psql", "-X", "-U", user, "-d", db_name,
            ]

    @staticmethod
    def variable_args(variables: dict[str, str]) -> list[str]:
        result: list[str] = []
        for name, variable_value in variables.items():
            result.extend(["-v", f"{name}={variable_value}"])
        return result

    def query(self, sql: str, variables: dict[str, str] | None = None) -> list[str]:
        command = self.command + self.variable_args(variables or {}) + ["-At", "-v", "ON_ERROR_STOP=1"]
        result = subprocess.run(command, input=sql, check=True, text=True, capture_output=True)
        return [line for line in result.stdout.splitlines() if line]

    def execute(self, sql: str, variables: dict[str, str] | None = None) -> None:
        command = self.command + self.variable_args(variables or {}) + ["-v", "ON_ERROR_STOP=1"]
        result = subprocess.run(command, input=sql, text=True, capture_output=True)
        if result.returncode != 0:
            stderr = result.stderr.strip() or result.stdout.strip()
            raise DemoError(f"Database execute error: {stderr}")

    def apply(self, path: Path, variables: dict[str, str]) -> None:
        command = self.command + self.variable_args(variables) + ["-v", "ON_ERROR_STOP=1"]
        result = subprocess.run(command, input=path.read_text(encoding="utf-8"), text=True, capture_output=True)
        if result.returncode != 0:
            stderr = result.stderr.strip() or result.stdout.strip()
            raise DemoError(f"Failed to apply {path.name}: {stderr}")


def ensure_health(origin: str) -> None:
    status, body = request_json(origin, "/health")
    if status != 200 or not isinstance(body, dict) or body.get("status") != "ok":
        raise DemoError(f"OpsWarden is not healthy at {origin} (HTTP {status})")


def ensure_user(origin: str, email: str, password: str) -> None:
    status, _ = request_json(
        origin, "/api/auth/sign-up", method="POST", payload={"email": email, "password": password}
    )
    if status not in {201, 409}:
        raise DemoError(f"Could not ensure demo user {email} (HTTP {status})")


def bootstrap_accounts(config: dict[str, str], target: str, origin: str) -> None:
    password = target_value(config, target, "PASSWORD")
    if not password:
        raise DemoError(f"Missing demo configuration: DEMO_{target.upper()}_PASSWORD or DEMO_PASSWORD")
    emails = [
        target_value(config, target, "RESPONDER_EMAIL"), target_value(config, target, "OBSERVER_EMAIL"),
        target_value(config, target, "CONTRACTOR_EMAIL"),
    ]
    if target == "local":
        emails.insert(0, target_value(config, target, "MANAGER_EMAIL"))
    if any(not email for email in emails):
        raise DemoError("The Manager, Responder, Observer and Contractor demo emails must be configured")
    for email in emails:
        ensure_user(origin, email, password)


def identity_variables(config: dict[str, str], database: Database, target: str) -> dict[str, str]:
    team_name = target_value(config, target, "TEAM_NAME")
    if not team_name:
        raise DemoError(f"Missing demo configuration: DEMO_{target.upper()}_TEAM_NAME or DEMO_TEAM_NAME")
    variables: dict[str, str] = {}
    for role in ("manager", "responder", "observer", "contractor"):
        email = target_value(config, target, f"{role.upper()}_EMAIL")
        if not email:
            raise DemoError(f"Missing demo configuration for the {role} email")
        rows = database.query("select id from users where email = :'email'", {"email": email})
        if len(rows) != 1:
            raise DemoError(f"Expected exactly one {role} user for {email}; found {len(rows)}")
        UUID(rows[0])
        variables[f"{role}_id"] = rows[0]
    teams = database.query(
        """select team.id from teams team join team_members member on member.team_id = team.id
           where team.name = :'team_name' and member.user_id = :'manager_id'::uuid
             and member.role = 'manager' order by team.id""",
        {"team_name": team_name, "manager_id": variables["manager_id"]},
    )
    configured_team = value(config, f"DEMO_{target.upper()}_TEAM_ID") or value(config, "DEMO_TEAM_ID")
    if configured_team:
        teams = [team for team in teams if team == configured_team]
    if len(teams) == 0 and target == "local":
        team_id = str(uuid4())
        database.execute(
            """begin;
               insert into teams (id, name, invitation_code, created_at) values (:'team_id'::uuid, :'team_name', 'DEMO-' || upper(substring(md5(random()::text) from 1 for 6)), now());
               insert into team_members (team_id, user_id, role, joined_at) values (:'team_id'::uuid, :'manager_id'::uuid, 'manager', now());
               commit;""",
            {"team_id": team_id, "team_name": team_name, "manager_id": variables["manager_id"]},
        )
        teams = [team_id]
    if len(teams) != 1:
        raise DemoError(
            f"Expected one managed Team named {team_name!r}; found {len(teams)}. "
            f"Complete the real onboarding first or set DEMO_{target.upper()}_TEAM_ID."
        )
    UUID(teams[0])
    variables["team_id"] = teams[0]
    return variables


def manager_token(config: dict[str, str], target: str, origin: str, prompt: bool) -> str:
    configured = target_value(config, target, "MANAGER_TOKEN")
    if configured:
        return configured
    if prompt:
        token = getpass.getpass("Manager bearer token: ").strip()
        if not token:
            raise DemoError("The prompted bearer token is empty")
        return token
    email = target_value(config, target, "MANAGER_EMAIL")
    password = target_value(config, target, "PASSWORD")
    if not email or not password:
        raise DemoError("Manager email and password must be configured")
    status, body = request_json(
        origin, "/api/auth/sign-in", method="POST",
        payload={"email": email, "password": password},
    )
    if status != 200 or not isinstance(body, dict) or not body.get("token"):
        raise DemoError("Manager password sign-in failed; OAuth accounts must use --prompt-token")
    return str(body["token"])


def configure_connection(origin: str, team_id: str, service: str, payload: dict[str, object], token: str) -> dict[str, object]:
    status, body = request_json(
        origin, f"/api/teams/{team_id}/service-connections/by-service/{service}",
        method="PUT", token=token, payload=payload,
    )
    if status != 200 or not isinstance(body, dict):
        raise DemoError(f"Could not configure {service} (HTTP {status}: {body})")
    return body


def upsert_rule(origin: str, team_id: str, definition: dict[str, object], token: str) -> None:
    status, body = request_json(origin, f"/api/teams/{team_id}/automation-rules", token=token)
    if status != 200 or not isinstance(body, list):
        raise DemoError(f"Could not list automation rules (HTTP {status})")
    existing = next((rule for rule in body if rule.get("name") == definition["name"]), None)
    if existing:
        status, _ = request_json(
            origin, f"/api/teams/{team_id}/automation-rules/{existing['id']}",
            method="PATCH", token=token, payload={**definition, "enabled": True},
        )
        expected = 200
    else:
        status, created = request_json(
            origin, f"/api/teams/{team_id}/automation-rules",
            method="POST", token=token, payload=definition,
        )
        if status == 201 and isinstance(created, dict):
            status, _ = request_json(
                origin, f"/api/teams/{team_id}/automation-rules/{created['id']}",
                method="PATCH", token=token, payload={"enabled": True},
            )
            expected = 200
        else:
            expected = 201
    if status != expected:
        raise DemoError(f"Could not upsert rule {definition['name']} (HTTP {status})")


def configure_integrations(config: dict[str, str], origin: str, team_id: str, token: str) -> None:
    require(
        config, "DEMO_GITHUB_WEBHOOK_SECRET", "DEMO_GITLAB_WEBHOOK_SECRET",
        "DEMO_GENERIC_WEBHOOK_SECRET", "DEMO_ALERTMANAGER_WEBHOOK_SECRET",
    )
    connections = {
        "github": configure_connection(origin, team_id, "github", {"webhook_signing_secret": value(config, "DEMO_GITHUB_WEBHOOK_SECRET"), "personal_token": None}, token),
        "gitlab": configure_connection(origin, team_id, "gitlab", {"webhook_signing_secret": value(config, "DEMO_GITLAB_WEBHOOK_SECRET")}, token),
        "generic": configure_connection(origin, team_id, "generic", {"webhook_signing_secret": value(config, "DEMO_GENERIC_WEBHOOK_SECRET")}, token),
        "alertmanager": configure_connection(origin, team_id, "alertmanager", {"webhook_signing_secret": value(config, "DEMO_ALERTMANAGER_WEBHOOK_SECRET")}, token),
    }
    http_endpoint = value(config, "DEMO_HTTP_ENDPOINT")
    if http_endpoint:
        connections["http"] = configure_connection(origin, team_id, "http", {"endpoint_url": http_endpoint}, token)
    smtp_transport = [value(config, name) for name in ("DEMO_SMTP_HOST", "DEMO_SMTP_USERNAME", "DEMO_SMTP_PASSWORD")]
    smtp_values = [*smtp_transport, value(config, "DEMO_EMAIL_FROM")]
    if any(smtp_transport) and not all(smtp_values):
        raise DemoError("SMTP configuration is partial; host, username, password and from address must be set together")
    if all(smtp_transport):
        connections["email"] = configure_connection(origin, team_id, "email", {
            "smtp_host": smtp_values[0], "smtp_port": value(config, "DEMO_SMTP_PORT", "587"),
            "smtp_username": smtp_values[1], "smtp_password": smtp_values[2], "from_address": smtp_values[3],
        }, token)

    repository = value(config, "DEMO_GITHUB_REPOSITORY", "your-github-org/opswarden-demo")
    github_filter = {
        "repository": repository, "workflow": value(config, "DEMO_GITHUB_WORKFLOW", "OpsWarden Demo CI"),
        "branch": value(config, "DEMO_GITHUB_BRANCH", "main"), "conclusion": "failure",
    }
    definitions: list[dict[str, object]] = [
        {"name": "Demo: GitHub CI failure creates an incident", "trigger_connection_id": connections["github"]["id"], "trigger_kind": "ci_failed", "trigger_config": github_filter, "reaction_kind": "create_incident", "reaction_connection_id": None, "reaction_config": {"severity": "high", "title": "GitHub CI failed: {{repository}} / {{workflow}}"}},
        {"name": "Demo: GitLab CI failure creates an incident", "trigger_connection_id": connections["gitlab"]["id"], "trigger_kind": "ci_failed", "trigger_config": {"repository": value(config, "DEMO_GITLAB_PROJECT", "your-gitlab-namespace/opswarden-demo"), "workflow": value(config, "DEMO_GITLAB_PIPELINE", "CI"), "branch": value(config, "DEMO_GITLAB_BRANCH", "main"), "conclusion": "failed"}, "reaction_kind": "create_incident", "reaction_connection_id": None, "reaction_config": {"severity": "high", "title": "GitLab CI failed: {{repository}} / {{workflow}}"}},
        {"name": "Demo: Generic deployment failure creates an incident", "trigger_connection_id": connections["generic"]["id"], "trigger_kind": "generic_event", "trigger_config": {"event_type": "deployment_failed", "source": "opswarden-demo", "severity": "critical"}, "reaction_kind": "create_incident", "reaction_connection_id": None, "reaction_config": {"severity": "critical", "title": "{{title}} ({{external_id}})"}},
        {"name": "Demo: Alertmanager firing creates an incident", "trigger_connection_id": connections["alertmanager"]["id"], "trigger_kind": "alert_firing", "trigger_config": {"severity": "critical", "receiver": "opswarden"}, "reaction_kind": "create_incident", "reaction_connection_id": None, "reaction_config": {"severity": "critical", "title": "Alertmanager: {{summary}}"}},
    ]
    if "http" in connections:
        definitions.append({"name": "Demo: GitHub CI failure sends HTTP", "trigger_connection_id": connections["github"]["id"], "trigger_kind": "ci_failed", "trigger_config": github_filter, "reaction_kind": "http_notify", "reaction_connection_id": connections["http"]["id"], "reaction_config": {"message": "OpsWarden demo: {{repository}} / {{workflow}} failed on {{branch}}"}})
    if "email" in connections:
        require(config, "DEMO_EMAIL_TO")
        definitions.append({"name": "Demo: GitHub CI failure sends email", "trigger_connection_id": connections["github"]["id"], "trigger_kind": "ci_failed", "trigger_config": github_filter, "reaction_kind": "email_notify", "reaction_connection_id": connections["email"]["id"], "reaction_config": {"to": value(config, "DEMO_EMAIL_TO"), "subject": "OpsWarden: {{workflow}} failed", "body": "{{repository}} failed on {{branch}}. Open OpsWarden for the incident timeline."}})
    for definition in definitions:
        upsert_rule(origin, team_id, definition, token)
    print("Configured connections: " + ", ".join(sorted(connections)))
    print(f"Enabled {len(definitions)} deterministic demo rules.")


def connection_ids(database: Database, team_id: str) -> dict[str, str]:
    rows = database.query(
        "select service || '=' || id from service_connections where team_id = :'team_id'::uuid order by service",
        {"team_id": team_id},
    )
    return dict(row.split("=", 1) for row in rows)


def post_webhook(origin: str, path: str, payload: dict[str, object], headers: dict[str, str]) -> None:
    status, body = request_json(origin, path, method="POST", payload=payload, headers=headers)
    if status != 202:
        raise DemoError(f"Webhook {path} failed (HTTP {status}: {body})")
    print(f"Accepted {path}: {body}")


def run_webhooks(config: dict[str, str], origin: str, database: Database, team_id: str) -> None:
    require(
        config, "DEMO_GITHUB_WEBHOOK_SECRET", "DEMO_GITLAB_WEBHOOK_SECRET",
        "DEMO_GENERIC_WEBHOOK_SECRET", "DEMO_ALERTMANAGER_WEBHOOK_SECRET",
    )
    ids = connection_ids(database, team_id)
    missing = [service for service in ("github", "gitlab", "generic", "alertmanager") if service not in ids]
    if missing:
        raise DemoError("Missing configured connections: " + ", ".join(missing))
    repository = value(config, "DEMO_GITHUB_REPOSITORY", "your-github-org/opswarden-demo")
    github = {"repository": {"full_name": repository}, "workflow_run": {"name": value(config, "DEMO_GITHUB_WORKFLOW", "OpsWarden Demo CI"), "head_branch": value(config, "DEMO_GITHUB_BRANCH", "main"), "conclusion": "failure", "html_url": f"https://github.com/{repository}/actions/runs/42"}}
    encoded = json.dumps(github, separators=(",", ":")).encode()
    signature = hmac.new(value(config, "DEMO_GITHUB_WEBHOOK_SECRET").encode(), encoded, hashlib.sha256).hexdigest()
    post_webhook(origin, f"/webhooks/github/{ids['github']}", github, {"X-GitHub-Delivery": str(uuid4()), "X-GitHub-Event": "workflow_run", "X-Hub-Signature-256": f"sha256={signature}"})
    gitlab_project = value(config, "DEMO_GITLAB_PROJECT", "your-gitlab-namespace/opswarden-demo")
    gitlab = {"object_kind": "pipeline", "object_attributes": {"status": "failed", "ref": value(config, "DEMO_GITLAB_BRANCH", "main"), "name": value(config, "DEMO_GITLAB_PIPELINE", "CI"), "url": f"https://gitlab.com/{gitlab_project}/-/pipelines/42"}, "project": {"path_with_namespace": gitlab_project}}
    post_webhook(origin, f"/webhooks/gitlab/{ids['gitlab']}", gitlab, {"X-Gitlab-Event-UUID": str(uuid4()), "X-Gitlab-Event": "Pipeline Hook", "X-Gitlab-Token": value(config, "DEMO_GITLAB_WEBHOOK_SECRET")})
    generic = {"source": "opswarden-demo", "title": "Production deployment failed", "message": "Health check timed out", "severity": "critical", "external_id": "deploy-42", "event_url": "https://app.opswarden.dev"}
    post_webhook(origin, f"/webhooks/generic/{ids['generic']}", generic, {"X-OpsWarden-Delivery": str(uuid4()), "X-OpsWarden-Event": "deployment_failed", "X-OpsWarden-Token": value(config, "DEMO_GENERIC_WEBHOOK_SECRET")})
    alertmanager = {"version": "4", "groupKey": '{}:{severity="critical"}', "status": "firing", "receiver": "opswarden", "commonLabels": {"severity": "critical"}, "alerts": [{"status": "firing", "fingerprint": f"demo-{uuid4().hex[:12]}", "startsAt": "2026-08-30T19:00:00Z", "endsAt": "2030-01-01T00:00:00Z", "generatorURL": "https://prometheus.example/graph", "labels": {"alertname": "PaymentApiDown", "severity": "critical", "service": "payments"}, "annotations": {"summary": "Payment API unavailable", "description": "Health probe failed"}}]}
    post_webhook(origin, f"/webhooks/alertmanager/{ids['alertmanager']}", alertmanager, {"Authorization": f"Bearer {value(config, 'DEMO_ALERTMANAGER_WEBHOOK_SECRET')}"})
    time.sleep(2)
    counts = database.query("select status || '=' || count(*) from automation_runs run join automation_rules rule on rule.id = run.rule_id where rule.team_id = :'team_id'::uuid and rule.name like 'Demo: %' group by status order by status", {"team_id": team_id})
    print("Demo run status: " + (", ".join(counts) if counts else "no runs persisted yet"))


def doctor(config: dict[str, str], args: argparse.Namespace) -> None:
    origin = api_origin(config, args.target)
    ensure_health(origin)
    database = Database(config, args.target)
    variables = identity_variables(config, database, args.target)
    print(f"API healthy: {origin}")
    print(f"Team ready: {target_value(config, args.target, 'TEAM_NAME')} ({variables['team_id']})")
    print("Users ready: manager, responder, observer, contractor")
    optional = {
        "HTTP": bool(value(config, "DEMO_HTTP_ENDPOINT")),
        "SMTP": all(value(config, name) for name in ("DEMO_SMTP_HOST", "DEMO_SMTP_USERNAME", "DEMO_SMTP_PASSWORD", "DEMO_EMAIL_FROM", "DEMO_EMAIL_TO")),
    }
    print("Optional outbound: " + ", ".join(f"{name}={'ready' if ready else 'not configured'}" for name, ready in optional.items()))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("doctor", "bootstrap", "seed", "integrations", "run", "deseed"))
    parser.add_argument("--target", choices=("local", "production"), default="local")
    parser.add_argument("--confirm", default="", help="Required explicit production confirmation")
    parser.add_argument("--prompt-token", action="store_true", help="Securely prompt for an OAuth Manager bearer token")
    parser.add_argument("--data-only", action="store_true", help="Seed relational data without connections or rules")
    args = parser.parse_args()
    if args.data_only and args.command != "seed":
        raise DemoError("--data-only is valid only with the seed command")
    if args.command in {"bootstrap", "seed", "integrations", "deseed"}:
        require_confirmation(args, args.command)
    config = load_config()
    origin = api_origin(config, args.target)
    ensure_health(origin)
    database = Database(config, args.target)

    if args.command == "doctor":
        doctor(config, args)
        return 0
    if args.command == "bootstrap":
        bootstrap_accounts(config, args.target, origin)
        print("Demo accounts are ready. Complete the Manager's real Team onboarding before seed.")
        return 0
    if args.command == "seed":
        bootstrap_accounts(config, args.target, origin)
        variables = identity_variables(config, database, args.target)
        token = ""
        if not args.data_only:
            require(
                config, "DEMO_GITHUB_WEBHOOK_SECRET", "DEMO_GITLAB_WEBHOOK_SECRET",
                "DEMO_GENERIC_WEBHOOK_SECRET", "DEMO_ALERTMANAGER_WEBHOOK_SECRET",
            )
            token = manager_token(config, args.target, origin, args.prompt_token)
        database.apply(ASSETS / "seed.sql", variables)
        print(f"Seeded one Team: {target_value(config, args.target, 'TEAM_NAME')} ({variables['team_id']})")
        if not args.data_only:
            configure_integrations(config, origin, variables["team_id"], token)
        return 0
    variables = identity_variables(config, database, args.target)
    if args.command == "integrations":
        configure_integrations(config, origin, variables["team_id"], manager_token(config, args.target, origin, args.prompt_token))
    elif args.command == "run":
        run_webhooks(config, origin, database, variables["team_id"])
    elif args.command == "deseed":
        database.apply(ASSETS / "deseed.sql", variables)
        print(f"Removed deterministic demo data from Team {variables['team_id']}; Team, users and connections were preserved.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (DemoError, subprocess.CalledProcessError, ValueError) as error:
        print(f"demo: {error}", file=sys.stderr)
        raise SystemExit(1)
