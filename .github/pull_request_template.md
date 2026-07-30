## Summary

<!-- What changed, and why is it needed? Keep this short and concrete. -->

## Scope

- Milestone or issue:
- Core or Extended:

## Validation

<!-- Check what applies. Explain any relevant item left unchecked. -->

- [ ] Rust formatting and lint pass.
- [ ] Web lint, formatting and type checks pass.
- [ ] Relevant tests cover the main flow and at least one failure path.
- [ ] Changed source files stay at or below 500 lines.
- [ ] No type/lint bypass (`as any`, `@ts-ignore`, disabled dead-code checks) was added.
- [ ] Tests were not removed merely to satisfy lint or CI.
- [ ] User-facing or contract changes are documented.
- [ ] No secret or generated build output is included.

## Architecture check

- [ ] Business rules remain in `server/src/domain` or `server/src/app`.
- [ ] Handlers and clients only translate, validate or present data.
- [ ] The change is focused enough to squash into one descriptive commit.

## Evidence

<!-- Paste concise test output, screenshots or links that help reviewers verify the change. -->
