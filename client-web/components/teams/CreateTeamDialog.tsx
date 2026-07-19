import React, { useState } from "react";
import { Plus, X } from "lucide-react";
import { useCreateTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";
import { Button, IconButton } from "@/components/ui/Button";

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
      <Button onClick={() => setOpen(true)} variant="primary">
        <Plus className="h-4 w-4" />
        {t("createTeam")}
      </Button>

      {open && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="surface relative w-full max-w-md rounded-md p-6 shadow-2xl">
            <IconButton
              onClick={() => setOpen(false)}
              className="absolute top-3 right-3"
              label="Close dialog"
              size="sm"
              variant="ghost"
            >
              <X className="h-5 w-5" />
            </IconButton>
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
                <Button size="lg" onClick={() => setOpen(false)}>
                  {t("cancel")}
                </Button>
                <Button
                  type="submit"
                  disabled={createTeam.isPending || !name.trim()}
                  loading={createTeam.isPending}
                  size="lg"
                  variant="primary"
                >
                  {createTeam.isPending ? t("creating") : t("create")}
                </Button>
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
