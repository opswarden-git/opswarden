import { getRequestConfig } from "next-intl/server";
import en from "../messages/en.json";
import fr from "../messages/fr.json";
import { isAppLocale, type AppLocale } from "./locales";
import { routing } from "./routing";

/**
 * Catalogues are imported statically rather than through
 * `import(`../messages/${locale}.json`)`. A dynamic import keyed by an
 * expression resolves to a glob in the dev module graph, and editing one of the
 * JSON files does not reliably invalidate the cached module: the server keeps
 * serving a snapshot taken before the edit, so a key that exists on disk
 * reaches the client as `MISSING_MESSAGE` until the dev server is restarted.
 * Two catalogues are small enough that loading both costs nothing, and this
 * also removes one dynamic import per request.
 */
const catalogues = { en, fr } as Record<AppLocale, typeof en>;

export default getRequestConfig(async ({ requestLocale }) => {
  const requested = await requestLocale;
  const locale = requested && isAppLocale(requested) ? requested : routing.defaultLocale;

  return { locale, messages: catalogues[locale] };
});
