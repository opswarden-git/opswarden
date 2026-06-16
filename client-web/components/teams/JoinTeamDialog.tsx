import React, { useState } from "react";
import { Users, X } from "lucide-react";
import { useJoinTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";

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
      <button
        onClick={() => setOpen(true)}
        className="text-gold hover:text-gold-hover flex items-center gap-2 font-sans text-sm font-bold transition-colors"
      >
        <Users className="h-4 w-4" />
        {t("joinTeam")}
      </button>

      {open && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="glass relative w-full max-w-md rounded-xl p-6 shadow-2xl">
            <button
              onClick={() => setOpen(false)}
              className="text-muted hover:text-text absolute top-4 right-4"
              aria-label="Close dialog"
            >
              <X className="h-5 w-5" />
            </button>
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
                  className="focus:border-gold w-full rounded-md border border-white/10 bg-black/50 px-3 py-2 font-mono text-sm tracking-widest text-white uppercase focus:outline-none"
                  placeholder="OPS-XXXXXX"
                />
              </div>
              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setOpen(false)}
                  className="text-muted hover:text-text px-4 py-2 text-sm font-medium transition-colors"
                >
                  {t("cancel")}
                </button>
                <button
                  type="submit"
                  disabled={joinTeam.isPending || !code.trim()}
                  className="bg-gold hover:bg-gold-hover text-bg rounded-md px-4 py-2 font-sans text-sm font-bold transition-colors disabled:opacity-50"
                >
                  {joinTeam.isPending ? t("joining") : t("join")}
                </button>
              </div>
              {joinTeam.isError && (
                <p className="mt-2 text-sm text-red-500">{joinTeam.error.message}</p>
              )}
            </form>
          </div>
        </div>
      )}
    </>
  );
}
