# Official Sources

This file records the official assignment sources used by OpsWarden and how the
Markdown documentation derives from them.

## Source Of Truth

The files below are the primary sources. They must be kept in the repository as
immutable reference material.

| File                              | Role                                                                                                                                | SHA-256                                                            |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `docs/pdf/01_consignes_RTC.pdf`   | RTC 1 original project brief. Historically acquired, but its functional and quality criteria remain part of the cumulative audit.   | `f6d4764d7beaec3a2d42e82872852cf40571c810b26c1489e618aa97ded8cae5` |
| `docs/pdf/02_consignes_RTC.pdf`   | RTC 2 extension brief. Historically acquired, but its extension criteria remain part of the cumulative audit.                       | `d2c34294a432f86c9cfdf2e7e51a29be78c6463adb9ccbf613e8b8044ccf3132` |
| `docs/pdf/03_consignes_VIGIL.pdf` | Current VIGIL rattrapage brief. This is the active project frame and constrains how RTC 1 and RTC 2 achievements are reimplemented. | `6d2b8f9006b3191d475209add69caf875fa714afa4326a68d6d3c69e2a31fa53` |

If a Markdown file and a PDF disagree, the PDF wins until the Markdown is
corrected.

## Derived Markdown

The Markdown files are operational documents. They are easier to search, audit
and update than PDFs, but they are not the official source.

| Markdown file                       | Derived from                                                | Purpose                                                                                           |
| ----------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `docs/markdown/grading-criteria.md` | RTC 1 rubric, RTC 2 rubric, VIGIL brief                     | Cumulative audit matrix: what is OK, partial, KO, or not yet applicable.                          |
| `docs/markdown/consignes_VIGIL.md`  | `03_consignes_VIGIL.pdf`                                    | Searchable mirror of the VIGIL brief.                                                             |
| `docs/markdown/blueprint.md`        | All three PDFs plus current code architecture               | Product and technical target, with deferred/non-core ambitions separated from mandatory criteria. |
| `docs/markdown/roadmap.md`          | All three PDFs plus `grading-criteria.md`                   | Execution plan ordered by mandatory criteria first.                                               |
| `docs/markdown/WEBSOCKET_SPEC.md`   | Current server/client code plus VIGIL realtime expectations | Honest current WebSocket contract and known gaps.                                                 |
| `docs/markdown/UI_GUIDELINES.md`    | Current Next.js code plus VIGIL UI expectations             | Honest current UI conventions and known gaps.                                                     |

## Cumulative Rule

VIGIL is not a clean-room replacement for RTC 1 and RTC 2. The final project is
audited as:

```text
RTC 1 required achievements
+ RTC 2 required achievements
+ VIGIL-specific constraints
= cumulative final scope
```

The current VIGIL brief can rename the product domain, stack, and constraints.
For example, RTC vocabulary maps into OpsWarden as follows:

| RTC wording        | OpsWarden/VIGIL wording                 |
| ------------------ | --------------------------------------- |
| Server/community   | Team                                    |
| Channel            | Incident room / operational context     |
| Message            | Timeline entry / incident communication |
| Online users       | Team or incident presence               |
| Typing status      | Incident typing status                  |
| Owner/Admin/Member | Manager/Responder/Observer              |

## Update Policy

When an official PDF changes:

1. Replace or add the PDF in `docs/pdf/`.
2. Recompute the SHA-256 with:

   ```bash
   sha256sum docs/pdf/*.pdf
   ```

3. Update this file.
4. Re-audit the derived Markdown files, especially `grading-criteria.md`,
   `blueprint.md`, `roadmap.md`, and `consignes_VIGIL.md`.
5. Do not treat deployment, marketing website, AI/RAG, or portfolio-only work as
   priority unless the cumulative mandatory matrix is green or explicitly
   waived.

## Optional Text Extraction

For local audit/search work, the PDFs can be extracted with:

```bash
nix shell nixpkgs#poppler-utils -c sh -lc 'for f in docs/pdf/*.pdf; do pdftotext -layout "$f" "/tmp/$(basename "$f" .pdf).txt"; done'
```

The extracted text is a convenience artifact only. The PDFs remain the official
source.
