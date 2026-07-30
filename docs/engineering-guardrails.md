# Engineering guardrails

These rules protect `main` and keep feature work reviewable.

## Protected `main`

GitHub branch protection must remain enabled with:

- changes merged through a pull request;
- `CI · Required gate` required on an up-to-date branch;
- protection enforced for administrators;
- conversations resolved before merge;
- linear history required;
- force-pushes and branch deletion disabled.

The approving-review count is intentionally zero while the repository is
maintained solo. Increase it when another regular reviewer is available.

## Source hygiene

`tooling/check_source_hygiene.sh` is part of the workflow validation job.

- New Rust, TypeScript, JavaScript and shell files may not exceed 500 lines.
- No tracked owned source file currently exceeds 500 lines. Any file crossing
  that ceiling fails the required gate.
- `as any`, `@ts-ignore` and `@ts-nocheck` are forbidden in application code.
- Test files may not be deleted inside an unrelated change. Move or replace
  tests in a dedicated refactor instead.

The line cap is a ceiling, not a target. Prefer modules around 200–350 lines
when they represent a coherent responsibility.

Every Docker `FROM` reference is pinned to a full `sha256` manifest digest.
`tooling/check_dockerfile_pins.sh` rejects mutable base tags in CI.

## Feature acceptance

Every integration must cover:

- the successful end-to-end path;
- missing and invalid authentication;
- malformed input;
- non-triggering provider states;
- retry/idempotency behavior;
- the server-owned catalog contract and user-facing translations.

Do not remove fixtures or helpers because a linter reveals that their intended
test is missing. Finish the test or remove the unfinished feature from the PR.

New behavior tests should use a clear given/when/then structure, name the
business rule they protect and verify durable effects rather than only an HTTP
status. Sensitive paths need at least one negative case.

### Targeted mutation testing

The Alertmanager trust boundary has a reproducible campaign:

```bash
cargo install --locked cargo-mutants --version 27.1.0
just test-mutations-alertmanager
```

The campaign is intentionally limited to the Alertmanager parser/idempotence
and bounded outcome metrics. It runs library tests only, with two jobs and a
120-second timeout per mutant. Every surviving, non-viable or timed-out mutant
must be investigated; the command is green only when tests catch every viable
mutation.

### Forward-only migrations

Released migration files are immutable. Every new SQL file declares exactly one
phase on its own line:

```sql
-- opswarden: migration-phase=expand
-- opswarden: migration-phase=backfill
-- opswarden: migration-phase=contract
```

Expand and backfill phases may not contain destructive or narrowing DDL.
Contract migrations are isolated to a later release after a compatibility
window and verified backup. CI enforces the marker, migration immutability and
the destructive-SQL boundary with `tooling/check_migration_policy.sh`.

## Release ordering

Prepare version changes in a dedicated pull request. After the required gate is
green:

1. merge the release PR into `main`;
2. fetch and verify the resulting `main` commit;
3. create the annotated version tag on that merged commit;
4. push the tag and monitor the Release workflow through artifact publication.

Never tag the release branch before it is merged. Squash merging changes the
commit ID, and the workflow intentionally rejects a tag that is not reachable
from `main`.
