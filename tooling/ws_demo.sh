#!/usr/bin/env bash
# Live WebSocket smoke helper. Run it AFTER the server is up (see terminal 1).
# It creates a user + team + incident and prints copy-paste-ready commands:
#   - the wscat auth line (terminal 2)
#   - the trigger curls (terminal 3)
# Requires: curl, jq. Server assumed at http://localhost:8080.
set -euo pipefail
BASE=${BASE:-http://localhost:8080}
EMAIL="ws.$(date +%s)@opswarden.test"
PW=password123

curl -s -X POST "$BASE/api/auth/sign-up" -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PW\"}" >/dev/null
TOKEN=$(curl -s -X POST "$BASE/api/auth/sign-in" -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PW\"}" | jq -r .token)
USER_ID=$(curl -s "$BASE/api/me" -H "Authorization: Bearer $TOKEN" | jq -r .id)
TEAM_ID=$(curl -s -X POST "$BASE/api/teams" -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"name":"WS Demo"}' | jq -r .team_id)
INC_ID=$(curl -s -X POST "$BASE/api/incidents" -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"team_id\":\"$TEAM_ID\",\"title\":\"WS demo incident\",\"severity\":\"high\"}" | jq -r .incident_id)

cat <<OUT

================  WS DEMO READY  ================
user=$USER_ID  team=$TEAM_ID  incident=$INC_ID

--- TERMINAL 2 : connect, then paste the auth line ---
npx wscat -c ws://localhost:8080/ws

{"type":"auth","token":"$TOKEN"}

--- TERMINAL 3 : trigger events (watch them pop in wscat) ---
# 1) status change -> incident_state_changed
curl -s -X PUT $BASE/api/incidents/$INC_ID/status -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{"status":"acknowledged"}'

# 2) escalate -> incident_state_changed + incident_escalated
curl -s -X PUT $BASE/api/incidents/$INC_ID/status -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{"status":"escalated"}'

# 3) timeline entry -> timeline_entry_added
curl -s -X POST $BASE/api/incidents/$INC_ID/timeline -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{"content":"hello from curl"}'

# 4) self-assign (manager>=responder) -> incident_assigned
curl -s -X PUT $BASE/api/incidents/$INC_ID/assign -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d "{\"assignee_id\":\"$USER_ID\"}"
=================================================
OUT
