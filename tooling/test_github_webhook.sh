#!/usr/bin/env bash
set -euo pipefail

api_origin=${OPSWARDEN_API_ORIGIN:-http://localhost:8080}
webhook_secret=${OPSWARDEN_DEMO_WEBHOOK_SECRET:-opswarden-demo-webhook-secret}
connection_id=${OPSWARDEN_CONNECTION_ID:-}
automation_team_id=${OPSWARDEN_DEMO_TEAM_ID:-39aa8884-22cc-4764-a9e7-7df7c7619ba6}
demo_email=${OPSWARDEN_DEMO_EMAIL:-manager@opswarden.local}
demo_password=${OPSWARDEN_DEMO_PASSWORD:-sudo}
delivery_id=${OPSWARDEN_DELIVERY_ID:-$(uuidgen)}
body='{"repository":{"full_name":"opswarden/demo"},"workflow_run":{"name":"CI","head_branch":"main","conclusion":"failure","html_url":"https://github.com/opswarden/demo/actions/runs/42"}}'
signature=$(printf '%s' "$body" | openssl dgst -sha256 -hmac "$webhook_secret" -hex | awk '{print $2}')

if [[ -z "$connection_id" ]]; then
  token=$(curl --fail --silent \
    --request POST "$api_origin/api/auth/sign-in" \
    --header 'Content-Type: application/json' \
    --data "$(jq -nc --arg email "$demo_email" --arg password "$demo_password" \
      '{email: $email, password: $password}')" \
    | jq -er '.token')
  connection_id=$(curl --fail --silent \
    "$api_origin/api/teams/$automation_team_id/service-connections" \
    --header "Authorization: Bearer $token" \
    | jq -er '.[] | select(.service == "github") | .id')
fi
webhook_url="$api_origin/webhooks/github/$connection_id"

curl --fail --silent --show-error \
  --request POST "$webhook_url" \
  --header 'Content-Type: application/json' \
  --header "X-Hub-Signature-256: sha256=$signature" \
  --header 'X-GitHub-Event: workflow_run' \
  --header "X-GitHub-Delivery: $delivery_id" \
  --data-binary "$body"
echo
