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
const DOCS_OUT = process.env.OPSWARDEN_INVENTORY_DOCS_DIR;
const API = process.env.OPSWARDEN_API_URL ?? "http://localhost:8080";

async function main() {
  fs.rmSync(OUT, { recursive: true, force: true });
  fs.mkdirSync(OUT, { recursive: true });
  if (DOCS_OUT) {
    fs.rmSync(DOCS_OUT, { recursive: true, force: true });
    fs.mkdirSync(DOCS_OUT, { recursive: true });
  }

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

  const write = (slug, rendered) => {
    fs.writeFileSync(path.join(OUT, `${slug}.html`), rendered.html);
    if (DOCS_OUT) fs.writeFileSync(path.join(DOCS_OUT, `${slug}.md`), rendered.markdown);
  };

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

  write(
    "index",
    renderIndex([
      {
        slug: "capabilities",
        subFr: "Toute l’autorisation produit sur une grille.",
        subEn: "The whole product authorisation on one grid.",
      },
      {
        slug: "conversations",
        subFr: "Parité entre messagerie directe et war room.",
        subEn: "Parity between direct messages and the war room.",
      },
      {
        slug: "automations",
        subFr: catalog.available
          ? "Lu depuis le serveur en fonctionnement."
          : "Serveur injoignable.",
        subEn: catalog.available ? "Read from the running server." : "Server unreachable.",
      },
    ]),
  );

  console.log(`inventory: ${fs.readdirSync(OUT).length} pages -> tooling/inventory/dist`);
}

await main();
