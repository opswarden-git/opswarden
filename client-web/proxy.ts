import createMiddleware from "next-intl/middleware";
import { appLocales } from "./i18n/locales";

export default createMiddleware({
  locales: appLocales as unknown as string[],
  defaultLocale: "en",
});

export const config = {
  matcher: ["/", "/(fr|en)/:path*", "/login", "/signup"],
};
