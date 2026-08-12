"use client";

import Image from "next/image";
import { Link } from "@/i18n/routing";
import { useTranslations } from "next-intl";
import { TeamSwitcher } from "@/components/teams/TeamSwitcher";
import { useTeamScope } from "@/components/teams/TeamScope";

/** Keeps product and Team context visible when the desktop sidebar disappears. */
export function MobileHeader() {
  const t = useTranslations("Sidebar");
  const { activeTeam, hrefFor } = useTeamScope();

  return (
    <header className="border-border bg-bg/95 sticky top-0 z-40 flex min-h-16 items-center gap-3 border-b px-4 py-2 backdrop-blur-md md:hidden">
      <Link
        href={activeTeam ? hrefFor("overview") : "/teams"}
        className="shrink-0"
        aria-label={t("logoWordmarkAlt")}
      >
        <Image
          src="/assets/logo-icon.png"
          alt=""
          width={30}
          height={25}
          className="h-auto w-auto"
          priority
        />
      </Link>
      <TeamSwitcher compact className="ml-auto min-w-0 flex-1 justify-end" />
    </header>
  );
}
