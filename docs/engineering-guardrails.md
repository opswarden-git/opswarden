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
- Existing files above 500 lines are grandfathered, but any changed oversized
  file must stay the same size or shrink.
- `as any`, `@ts-ignore` and `@ts-nocheck` are forbidden in application code.
- Test files may not be deleted inside an unrelated change. Move or replace
  tests in a dedicated refactor instead.

The line cap is a ceiling, not a target. Prefer modules around 200–350 lines
when they represent a coherent responsibility.

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
