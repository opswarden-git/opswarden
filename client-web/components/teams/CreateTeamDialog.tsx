import React, { useState } from "react";
import { Plus, X } from "lucide-react";
import { useCreateTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";

export function CreateTeamDialog() {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const createTeam = useCreateTeam();
  const t = useTranslations("Teams");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    createTeam.mutate(name.trim(), {
      onSuccess: () => {
        setOpen(false);
        setName("");
      },
    });
  };

  return (
    <>
      <button
        onClick={() => setOpen(true)}
        className="ow-primary flex inline-flex h-9 items-center justify-center gap-2 rounded-md px-3.5 text-sm font-medium transition-colors"
      >
        <Plus className="h-4 w-4" />
        {t("createTeam")}
      </button>

      {open && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="surface relative w-full max-w-md rounded-md p-6 shadow-2xl">
            <button
              onClick={() => setOpen(false)}
              className="text-muted hover:text-text absolute top-4 right-4"
              aria-label="Close dialog"
            >
              <X className="h-5 w-5" />
            </button>
            <h2 className="text-text mb-4 font-sans text-lg font-bold">{t("createTitle")}</h2>
            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <label htmlFor="team-name" className="text-muted mb-1 block font-sans text-xs">
                  {t("name")}
                </label>
                <input
                  id="team-name"
                  type="text"
                  autoFocus
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
                  placeholder="e.g. NOC-Alpha"
                />
              </div>
              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setOpen(false)}
                  className="ow-secondary inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors"
                >
                  {t("cancel")}
                </button>
                <button
                  type="submit"
                  disabled={createTeam.isPending || !name.trim()}
                  className="ow-primary inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors disabled:opacity-50"
                >
                  {createTeam.isPending ? t("creating") : t("create")}
                </button>
              </div>
              {createTeam.isError && (
                <p className="text-sev-critical mt-2 text-sm">{createTeam.error.message}</p>
              )}
            </form>
          </div>
        </div>
      )}
    </>
  );
}
