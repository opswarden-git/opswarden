# Team integrations

OpsWarden separates identity OAuth from Team automation connections. Signing in
with GitHub identifies a user; authorizing GitHub inside a Team stores a distinct
encrypted grant for that Team's automation capabilities.

The reproducible presentation path is documented in
[Demo dataset and integration runs](DEMO_DATASET.md). In short:

1. create the Team through onboarding;
2. configure the `DEMO_` values in the ignored `.env` file;
3. run `python3 tooling/demo.py seed --target local` to replace the Team's
   presentation data and configure its deterministic rules;
4. exercise one signed event per inbound provider with
   `python3 tooling/demo.py run --target local`.

For the Alertmanager payload and authentication contract, see
[Alertmanager](ALERTMANAGER.md).

Production mutations require an explicit operation-specific confirmation. The
seed preserves the Team, Manager account, users and service connections, but
replaces the selected Team's presentation content.
