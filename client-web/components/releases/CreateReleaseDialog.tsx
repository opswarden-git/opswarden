import React, { useState } from "react";
import { Rocket, X } from "lucide-react";
import { useCreateRelease } from "@/lib/queries/releases";
import { useTranslations } from "next-intl";

/** Parse the comma-separated steps field into an ordered, de-duplicated list. */
function parseSteps(raw: string): string[] {
  const seen = new Set<string>();
  const steps: string[] = [];
  for (const part of raw.split(",")) {
    const name = part.trim();
    if (name && !seen.has(name)) {
      seen.add(name);
      steps.push(name);
    }
  }
  return steps;
}

export function CreateReleaseDialog({ teamId }: { teamId: string }) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [stepsRaw, setStepsRaw] = useState("");
  const createRelease = useCreateRelease();
  const t = useTranslations("Releases");
  const tErr = useTranslations("errors");

  const steps = parseSteps(stepsRaw);
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || steps.length === 0 || !teamId) return;
    createRelease.mutate(
      { team_id: teamId, title: title.trim(), steps },
      {
        onSuccess: () => {
          setOpen(false);
          setTitle("");
          setStepsRaw("");
        },
      },
    );
  };

  return (
    <>
      <button
        onClick={() => {
          createRelease.reset();
          setOpen(true);
        }}
        disabled={!teamId}
        className="ow-primary flex h-9 items-center gap-2 rounded-md px-3.5 text-sm font-medium transition-colors disabled:opacity-50"
      >
        <Rocket className="h-4 w-4" />
        {t("newRelease")}
      </button>

      {open && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="surface relative w-full max-w-lg rounded-md p-6 shadow-2xl">
            <button
              onClick={() => setOpen(false)}
              className="text-muted hover:text-text absolute top-4 right-4"
              aria-label={t("close")}
            >
              <X className="h-5 w-5" />
            </button>
            <div className="mb-6 flex items-center gap-3">
              <div className="bg-gold/15 text-gold flex h-10 w-10 items-center justify-center rounded-full">
                <Rocket className="h-5 w-5" />
              </div>
              <div>
                <h2 className="text-text font-sans text-lg font-bold">{t("newRelease")}</h2>
                <p className="text-muted text-xs">{t("newReleaseDesc")}</p>
              </div>
            </div>

            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <label htmlFor="rel-title" className="text-muted mb-1 block font-sans text-xs">
                  {t("releaseTitle")}
                </label>
                <input
                  id="rel-title"
                  type="text"
                  autoFocus
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
                  placeholder={t("titlePlaceholder")}
                />
              </div>

              <div>
                <label htmlFor="rel-steps" className="text-muted mb-1 block font-sans text-xs">
                  {t("steps")}
                </label>
                <input
                  id="rel-steps"
                  type="text"
                  value={stepsRaw}
                  onChange={(e) => setStepsRaw(e.target.value)}
                  className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
                  placeholder={t("stepsPlaceholder")}
                />
                <p className="text-muted/60 mt-1 text-xs">{t("stepsHint")}</p>
                {steps.length > 0 ? (
                  <div className="mt-2 flex flex-wrap gap-1.5">
                    {steps.map((step, i) => (
                      <span
                        key={step}
                        className="surface-subtle border-border text-muted rounded border px-1.5 py-0.5 text-xs"
                      >
                        {i + 1}. {step}
                      </span>
                    ))}
                  </div>
                ) : null}
              </div>

              <div className="flex justify-end gap-2 pt-4">
                <button
                  type="button"
                  onClick={() => setOpen(false)}
                  className="ow-secondary inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors"
                >
                  {t("cancel")}
                </button>
                <button
                  type="submit"
                  disabled={createRelease.isPending || !title.trim() || steps.length === 0}
                  className="ow-primary inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors disabled:opacity-50"
                >
                  {createRelease.isPending ? t("creating") : t("create")}
                </button>
              </div>
              {createRelease.isError ? (
                <p className="text-sev-critical mt-2 text-sm">
                  {errorText(createRelease.error.message)}
                </p>
              ) : null}
            </form>
          </div>
        </div>
      )}
    </>
  );
}
