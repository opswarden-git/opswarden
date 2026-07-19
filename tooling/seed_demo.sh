#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
api_origin=${OPSWARDEN_API_ORIGIN:-http://localhost:8080}
demo_password=${OPSWARDEN_DEMO_PASSWORD:-sudo}
webhook_secret=${OPSWARDEN_DEMO_WEBHOOK_SECRET:-opswarden-demo-webhook-secret}
automation_team_id=39aa8884-22cc-4764-a9e7-7df7c7619ba6

cd "$repo_dir"

if ! curl --fail --silent "$api_origin/health" >/dev/null; then
  echo "OpsWarden server is not reachable at $api_origin" >&2
  exit 1
fi

ensure_user() {
  local email=$1
  local status
  status=$(curl --silent --output /dev/null --write-out '%{http_code}' \
    --request POST "$api_origin/api/auth/sign-up" \
    --header 'Content-Type: application/json' \
    --data "{\"email\":\"$email\",\"password\":\"$demo_password\"}")
  if [[ "$status" != "201" && "$status" != "409" ]]; then
    echo "Could not ensure demo user $email (HTTP $status)" >&2
    exit 1
  fi
}

for email in \
  manager@opswarden.local \
  responder@opswarden.local \
  observer@opswarden.local \
  contractor@opswarden.local; do
  ensure_user "$email"
done

db_query() {
  docker compose exec -T db sh -lc \
    'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Atc "$1"' sh "$1"
}

manager_id=$(db_query "select id from users where email = 'manager@opswarden.local'")
responder_id=$(db_query "select id from users where email = 'responder@opswarden.local'")
observer_id=$(db_query "select id from users where email = 'observer@opswarden.local'")
contractor_id=$(db_query "select id from users where email = 'contractor@opswarden.local'")

docker compose exec -T db psql \
  -U opswarden -d opswarden \
  -v manager_id="$manager_id" \
  -v responder_id="$responder_id" \
  -v observer_id="$observer_id" \
  -v contractor_id="$contractor_id" \
  < tooling/seed_demo.sql

token=$(curl --fail --silent \
  --request POST "$api_origin/api/auth/sign-in" \
  --header 'Content-Type: application/json' \
  --data "{\"email\":\"manager@opswarden.local\",\"password\":\"$demo_password\"}" \
  | jq -er '.token')

github_connection=$(curl --fail --silent \
  --request PUT "$api_origin/api/teams/$automation_team_id/service-connections/github" \
  --header "Authorization: Bearer $token" \
  --header 'Content-Type: application/json' \
  --data "$(jq -nc --arg secret "$webhook_secret" '{webhook_signing_secret: $secret}')")
github_connection_id=$(jq -er '.id' <<<"$github_connection")

rules=$(curl --fail --silent \
  "$api_origin/api/teams/$automation_team_id/automation-rules" \
  --header "Authorization: Bearer $token")
demo_rule_id=$(jq -r '.[] | select(.name == "Demo: GitHub CI failure to incident") | .id' <<<"$rules")

if [[ -z "$demo_rule_id" ]]; then
  demo_rule=$(curl --fail --silent \
    --request POST "$api_origin/api/teams/$automation_team_id/automation-rules" \
    --header "Authorization: Bearer $token" \
    --header 'Content-Type: application/json' \
    --data "$(jq -nc --arg connection "$github_connection_id" '{
      name: "Demo: GitHub CI failure to incident",
      trigger_connection_id: $connection,
      trigger_kind: "ci_failed",
      trigger_config: {},
      reaction_kind: "vigil_create_incident",
      reaction_connection_id: null,
      reaction_config: {severity: "high"}
    }')")
  demo_rule_id=$(jq -er '.id' <<<"$demo_rule")
fi

curl --fail --silent \
  --request PATCH "$api_origin/api/teams/$automation_team_id/automation-rules/$demo_rule_id" \
  --header "Authorization: Bearer $token" \
  --header 'Content-Type: application/json' \
  --data '{"enabled":true}' \
  >/dev/null

echo
echo "Demo data ready."
echo "Accounts: manager/responder/observer @opswarden.local (password: $demo_password)"
echo "GitHub webhook secret: $webhook_secret"
echo "GitHub webhook endpoint: $api_origin/webhooks/github/$github_connection_id"
