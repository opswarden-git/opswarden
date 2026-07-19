import React, { useState } from "react";
import { Users, X } from "lucide-react";
import { useJoinTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";
import { Button, IconButton } from "@/components/ui/Button";

export function JoinTeamDialog() {
  const [open, setOpen] = useState(false);
  const [code, setCode] = useState("");
  const joinTeam = useJoinTeam();
  const t = useTranslations("Teams");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!code.trim()) return;
    joinTeam.mutate(code.trim(), {
      onSuccess: () => {
        setOpen(false);
        setCode("");
      },
    });
  };

  return (
    <>
      <Button onClick={() => setOpen(true)}>
        <Users className="h-4 w-4" />
        {t("joinTeam")}
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
            <h2 className="text-text mb-4 font-sans text-lg font-bold">{t("joinTitle")}</h2>
            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <label htmlFor="join-code" className="text-muted mb-1 block font-sans text-xs">
                  {t("colInvitationCode")}
                </label>
                <input
                  id="join-code"
                  type="text"
                  autoFocus
                  value={code}
                  onChange={(e) => setCode(e.target.value.toUpperCase())}
                  className="ow-input flex h-10 w-full rounded-md px-3 py-2 font-mono text-sm tracking-widest uppercase transition-colors"
                  placeholder="OPS-XXXXXX"
                />
              </div>
              <div className="flex justify-end gap-2 pt-2">
                <Button size="lg" onClick={() => setOpen(false)}>
                  {t("cancel")}
                </Button>
                <Button
                  type="submit"
                  disabled={joinTeam.isPending || !code.trim()}
                  loading={joinTeam.isPending}
                  size="lg"
                  variant="primary"
                >
                  {joinTeam.isPending ? t("joining") : t("join")}
                </Button>
              </div>
              {joinTeam.isError && (
                <p className="text-sev-critical mt-2 text-sm">{joinTeam.error.message}</p>
              )}
            </form>
          </div>
        </div>
      )}
    </>
  );
}
