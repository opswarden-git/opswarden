"use client";

import React from "react";
import Image from "next/image";
import { Languages } from "lucide-react";
import { useParams } from "next/navigation";
import { useRouter as useIntlRouter, usePathname } from "@/i18n/routing";
import { useTranslations } from "next-intl";

/** Interface language switch (FR/EN). */
export function LanguagePanel() {
  const t = useTranslations("Settings");
  const intlRouter = useIntlRouter();
  const pathname = usePathname();
  const params = useParams();
  const currentLocale = params.locale as string;

  const switchLocale = (newLocale: string) => {
    intlRouter.replace(pathname, { locale: newLocale });
  };

  return (
    <div className="surface rounded-md p-6">
      <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
        <Languages className="text-muted h-5 w-5" />
        {t("language")}
      </h2>
      <div className="mt-4 flex items-center justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-text text-sm font-medium">{t("interfaceLanguage")}</h3>
        </div>
        <div className="flex shrink-0 gap-4">
          <button
            onClick={() => switchLocale("en")}
            className={`ring-offset-bg overflow-hidden rounded-full ring-offset-2 transition-all ${
              currentLocale === "en"
                ? "ring-gold opacity-100 ring-2 grayscale-0"
                : "opacity-50 grayscale hover:opacity-100 hover:grayscale-0"
            }`}
          >
            <Image
              src="/assets/en.webp"
              alt="English"
              width={24}
              height={24}
              className="block object-cover"
            />
          </button>
          <button
            onClick={() => switchLocale("fr")}
            className={`ring-offset-bg overflow-hidden rounded-full ring-offset-2 transition-all ${
              currentLocale === "fr"
                ? "ring-gold opacity-100 ring-2 grayscale-0"
                : "opacity-50 grayscale hover:opacity-100 hover:grayscale-0"
            }`}
          >
            <Image
              src="/assets/fr.webp"
              alt="Français"
              width={24}
              height={24}
              className="block object-cover"
            />
          </button>
        </div>
      </div>
    </div>
  );
}
