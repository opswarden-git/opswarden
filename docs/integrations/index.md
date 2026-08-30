# Team integrations

OpsWarden separates identity OAuth from Team automation connections. Signing in
with GitHub identifies a user; authorizing GitHub inside a Team stores a distinct
encrypted grant for that Team's automation capabilities.

The reproducible presentation path is:

1. create the Team through onboarding;
2. run `python3 tooling/demo.py seed --target local`;
3. configure and enable the deterministic rules with
   `python3 tooling/demo.py integrations --target local`;
4. exercise one signed event per inbound provider with
   `python3 tooling/demo.py run --target local`.

For the Alertmanager payload and authentication contract, see
[Alertmanager](ALERTMANAGER.md).

Production mutations require an explicit operation-specific confirmation. The
seed preserves the Team, Manager account, users and service connections, but
replaces the selected Team's presentation content.
