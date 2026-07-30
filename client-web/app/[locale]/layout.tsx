import { NextIntlClientProvider } from "next-intl";
import { getMessages, getTranslations } from "next-intl/server";
import { notFound } from "next/navigation";
import type { Metadata, Viewport } from "next";
import { isAppLocale } from "@/i18n/locales";
import { routing } from "@/i18n/routing";
import { Inter, JetBrains_Mono } from "next/font/google";
import "../globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-sans" });
const jetbrainsMono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono" });
import { AppShell } from "@/components/layout/AppShell";

export const viewport: Viewport = {
  themeColor: "#15161A",
};

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const safeLocale = routing.locales.includes(locale as "en" | "fr") ? locale : "en";
  const t = await getTranslations({ locale: safeLocale, namespace: "Metadata" });
  return {
    applicationName: "OpsWarden",
    title: {
      default: "OpsWarden",
      template: "%s | OpsWarden",
    },
    description: t("description"),
    appleWebApp: {
      capable: true,
      statusBarStyle: "black-translucent",
    },
  };
}

import { Providers } from "@/app/providers";
import { AuthGuard } from "@/components/AuthGuard";

export default async function LocaleLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  // Ensure that the incoming `locale` is valid
  if (!isAppLocale(locale)) {
    notFound();
  }

  // Providing all messages to the client
  // side is the easiest way to get started
  const messages = await getMessages();

  return (
    <html lang={locale} className="dark">
      <body className={`dark font-sans ${inter.variable} ${jetbrainsMono.variable}`}>
        <NextIntlClientProvider messages={messages}>
          <Providers>
            <AuthGuard>
              <AppShell>{children}</AppShell>
            </AuthGuard>
          </Providers>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
