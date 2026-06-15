import { NextIntlClientProvider } from "next-intl";
import { getMessages } from "next-intl/server";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { routing } from "@/i18n/routing";
import "../globals.css";
import { AppShell } from "@/components/layout/AppShell";

export const metadata: Metadata = {
  title: {
    default: "OpsWarden",
    template: "%s | OpsWarden",
  },
  description: "Real-time incident management and operational coordination.",
};

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
  if (!routing.locales.includes(locale as any)) {
    notFound();
  }

  // Providing all messages to the client
  // side is the easiest way to get started
  const messages = await getMessages();

  return (
    <html lang={locale} className="dark">
      <body className="dark font-sans">
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
