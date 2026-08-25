// --- tooling/inventory/build.mjs ---
//
// Renders every inventory page into `dist/`. Run with `just inventory`.

import fs from "node:fs";
import path from "node:path";

import * as server from "./extract-server.mjs";
import * as web from "./extract-web.mjs";
import { renderApi, renderCapabilities, renderErrors, renderEvents } from "./render-foundation.mjs";
import { renderAutomations, renderConversations, renderData, renderUi } from "./render-product.mjs";
import { renderContracts, renderI18n, renderIndex } from "./render-quality.mjs";
import { ROOT } from "./sources.mjs";

const OUT = path.join(ROOT, "tooling/inventory/dist");
const API = process.env.OPSWARDEN_API_URL ?? "http://localhost:8080";

async function main() {
  fs.rmSync(OUT, { recursive: true, force: true });
  fs.mkdirSync(OUT, { recursive: true });

  const capabilities = server.capabilities();
  const errors = server.errors();
  const events = server.events();
  const ws = server.websocket();
  const routes = server.routes();
  const states = server.stateMachines();
  const limits = server.limits();
  const features = server.conversationFeatures();
  const migrations = server.migrations();

  const tokenData = web.tokens();
  const primitives = web.primitives();
  const attributes = web.domContract();
  const copy = web.i18n();
  const catalog = await web.automations(API);

  const write = (slug, html) => fs.writeFileSync(path.join(OUT, `${slug}.html`), html);

  // The badge board is hand-authored and stays that way: three of its seven
  // sources encode conditions rather than enum keys, so a generator would have
  // to invent the same branch names its author did. It is copied verbatim and
  // only gains a way back to the index.
  const badges = fs
    .readFileSync(path.join(ROOT, "tooling/inventory/static/badges.html"), "utf8")
    .replace(
      "<main>",
      '<main>\n<p style="margin-bottom:18px"><a href="index.html" ' +
        'style="color:#989ba1;font-size:12px;text-decoration:none">&#8592; ' +
        '<span data-fr="Inventaire" data-en="Inventory">Inventaire</span></a></p>',
    );
  fs.writeFileSync(path.join(OUT, "badges.html"), badges);

  write("capabilities", renderCapabilities(capabilities));
  write("errors", renderErrors(errors));
  write("api", renderApi(routes));
  write("events", renderEvents(events, ws));
  write("conversations", renderConversations(features, limits));
  write("data", renderData(migrations, states));
  write("automations", renderAutomations(catalog, API));
  write("ui", renderUi(primitives, tokenData));
  write("contracts", renderContracts(attributes));
  write("i18n", renderI18n(copy));

  const uncovered = attributes.filter((attribute) => !attribute.covered).length;
  write(
    "index",
    renderIndex([
      {
        slug: "capabilities",
        count: `${capabilities.roles.length}×${capabilities.fields.length}`,
        fr: "Rôles et capacités",
        en: "Roles and capabilities",
        subFr: "Toute l’autorisation produit sur une grille",
        subEn: "The whole product authorisation on one grid",
      },
      {
        slug: "errors",
        count: errors.rows.length,
        fr: "Erreurs",
        en: "Errors",
        subFr: `${errors.rows.filter((row) => !row.en).length} sans message d’interface`,
        subEn: `${errors.rows.filter((row) => !row.en).length} with no interface message`,
      },
      {
        slug: "api",
        count: routes.length,
        fr: "Routes HTTP",
        en: "HTTP routes",
        subFr: `${routes.filter((route) => !route.guarded).length} publiques, ${routes.filter((route) => route.bodyLimit).length} avec plafond explicite`,
        subEn: `${routes.filter((route) => !route.guarded).length} public, ${routes.filter((route) => route.bodyLimit).length} with an explicit ceiling`,
      },
      {
        slug: "events",
        count: `${events.length}+${ws.frames.length}`,
        fr: "Événements et trames",
        en: "Events and frames",
        subFr: "Portée de livraison et protocole temps réel",
        subEn: "Delivery scope and the realtime protocol",
      },
      {
        slug: "conversations",
        count: features.all.length,
        fr: "Capacités de conversation",
        en: "Conversation features",
        subFr: "Parité entre messagerie directe et war room",
        subEn: "Parity between direct messages and the war room",
      },
      {
        slug: "data",
        count: migrations.length,
        fr: "Migrations et états",
        en: "Migrations and states",
        subFr: "Registre forward-only et cycles de vie",
        subEn: "Forward-only ledger and lifecycles",
      },
      {
        slug: "automations",
        count: catalog.available ? catalog.services.length : "—",
        fr: "Services d’automation",
        en: "Automation services",
        subFr: catalog.available ? "Lu depuis le serveur en fonctionnement" : "Serveur injoignable",
        subEn: catalog.available ? "Read from the running server" : "Server unreachable",
      },
      {
        slug: "ui",
        count: primitives.length,
        fr: "Primitives d’interface",
        en: "Interface primitives",
        subFr: `${tokenData.total} jetons de design déclarés`,
        subEn: `${tokenData.total} declared design tokens`,
      },
      {
        slug: "contracts",
        count: attributes.length,
        fr: "Contrats DOM",
        en: "DOM contracts",
        subFr: `${uncovered} jamais sélectionnés par les E2E`,
        subEn: `${uncovered} never selected by the E2E suite`,
      },
      {
        slug: "i18n",
        count: copy.totalKeys,
        fr: "Clés de texte",
        en: "Copy keys",
        subFr: `${copy.namespaces.length} namespaces sous plafond`,
        subEn: `${copy.namespaces.length} namespaces under a ceiling`,
      },
      {
        slug: "badges",
        count: 28,
        fr: "Badges d’état",
        en: "State badges",
        subFr: "Écrite à la main — la seule planche qui ne se génère pas",
        subEn: "Hand-authored — the one board that is not generated",
      },
    ]),
  );

  console.log(`inventory: ${fs.readdirSync(OUT).length} pages -> tooling/inventory/dist`);
}

await main();
