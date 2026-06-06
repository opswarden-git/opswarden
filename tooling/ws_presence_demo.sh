#!/usr/bin/env bash
# Live PRESENCE smoke helper (WS-PR2). Run it AFTER the server is up.
# It creates TWO users in the SAME team plus one incident, then prints two
# copy-paste-ready wscat blocks. Open each block in its own terminal: after the
# `auth` line, send the `watch` line and watch `presence_update` frames flow as
# the other client joins / leaves.
# Requires: curl, jq. Server assumed at http://localhost:8080.
set -euo pipefail
BASE=${BASE:-http://localhost:8080}
PW=password123
STAMP=$(date +%s)
A="alice.$STAMP@opswarden.test"
B="bob.$STAMP@opswarden.test"

signup_signin() { # email -> token
  curl -s -X POST "$BASE/api/auth/sign-up" -H 'Content-Type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"$PW\"}" >/dev/null
  curl -s -X POST "$BASE/api/auth/sign-in" -H 'Content-Type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"$PW\"}" | jq -r .token
}

TOKEN_A=$(signup_signin "$A")
TOKEN_B=$(signup_signin "$B")
USER_A=$(curl -s "$BASE/api/me" -H "Authorization: Bearer $TOKEN_A" | jq -r .id)
USER_B=$(curl -s "$BASE/api/me" -H "Authorization: Bearer $TOKEN_B" | jq -r .id)

# Alice creates the team (she becomes Manager) and an incident.
TEAM_JSON=$(curl -s -X POST "$BASE/api/teams" -H "Authorization: Bearer $TOKEN_A" \
  -H 'Content-Type: application/json' -d '{"name":"Presence Demo"}')
TEAM_ID=$(echo "$TEAM_JSON" | jq -r .team_id)
INVITE=$(echo "$TEAM_JSON" | jq -r .invitation_code)
INC_ID=$(curl -s -X POST "$BASE/api/incidents" -H "Authorization: Bearer $TOKEN_A" \
  -H 'Content-Type: application/json' \
  -d "{\"team_id\":\"$TEAM_ID\",\"title\":\"Presence demo incident\",\"severity\":\"high\"}" | jq -r .incident_id)

# Bob joins the same team via the invitation code.
curl -s -X POST "$BASE/api/teams/join" -H "Authorization: Bearer $TOKEN_B" \
  -H 'Content-Type: application/json' -d "{\"invitation_code\":\"$INVITE\"}" >/dev/null

for v in TOKEN_A TOKEN_B USER_A USER_B TEAM_ID INC_ID; do
  [ -n "${!v}" ] && [ "${!v}" != "null" ] || { echo "FAILED to obtain $v (is the server up?)"; exit 1; }
done

cat <<OUT

==============  WS PRESENCE DEMO READY  ==============
team=$TEAM_ID  incident=$INC_ID
alice(user=$USER_A)   bob(user=$USER_B)

--- TERMINAL 2 : ALICE ---
npx wscat -c ws://localhost:8080/ws
{"type":"auth","token":"$TOKEN_A"}
{"type":"watch","incident_id":"$INC_ID"}

--- TERMINAL 3 : BOB ---
npx wscat -c ws://localhost:8080/ws
{"type":"auth","token":"$TOKEN_B"}
{"type":"watch","incident_id":"$INC_ID"}

Expected:
  - Alice watches first -> Alice receives presence_update watchers:[alice]
  - Bob watches         -> BOTH receive presence_update watchers:[alice,bob]
  - Ctrl-C on Bob (or send {"type":"unwatch","incident_id":"$INC_ID"})
                        -> Alice receives presence_update watchers:[alice]
=====================================================
OUT
