import { bi, card, escape, note, page, summary, table, tone, yesNo } from "./layout.mjs";
import { cell, key, muted, text } from "./render-helpers.mjs";

export function renderConversations(features, limits) {
  const featureRows = features.all.map((feature) => [
    key(feature, true),
    cell(yesNo(features.direct.includes(feature))),
    cell(yesNo(features.incident.includes(feature))),
  ]);
  const gaps = features.all.filter(
    (feature) => features.direct.includes(feature) !== features.incident.includes(feature),
  );

  return page({
    slug: "conversations",
    titleFr: "Conversations",
    titleEn: "Conversations",
    introFr:
      "Ce que chaque surface déclare savoir faire, et les bornes que le domaine impose des deux côtés. La colonne où les deux surfaces divergent est la seule qui mérite une justification.",
    introEn:
      "What each surface declares it can do, and the bounds the domain imposes on both. The rows where the two surfaces disagree are the only ones that need a justification.",
    body: `
${summary([
  [features.all.length, "capacités", "features"],
  [features.direct.length, "en messagerie directe", "on direct messages"],
  [features.incident.length, "en war room", "in the war room"],
  [gaps.length, "divergences", "divergences"],
  [limits.mediaTypes.length, "types MIME admis", "accepted MIME types"],
  [limits.reactions.length, "réactions", "reactions"],
])}
<section>
  <div class="section-head"><h2>${bi("Parité des surfaces", "Surface parity")}</h2></div>
  <div class="capture-grid">
    ${card(
      bi("ConversationFeature", "ConversationFeature"),
      "server/src/domain/conversation.rs",
      table(
        [bi("Capacité", "Feature"), bi("Directe", "Direct"), bi("Incident", "Incident")],
        featureRows,
        ["54%", "23%", "23%"],
      ) +
        note(
          "Les deux surfaces partagent désormais le même curseur keyset : une war room longue se remonte page par page, comme une conversation. Les deux divergences restantes, collaborative_cursors et system_events, sont légitimement propres à l’incident.",
          "Both surfaces now share the same keyset cursor: a long war room is walked back page by page, like a conversation. The two remaining divergences, collaborative_cursors and system_events, are legitimately incident-only.",
        ),
    )}
    ${card(
      bi("Bornes du domaine", "Domain bounds"),
      "server/src/domain/",
      table(
        [bi("Règle", "Rule"), bi("Valeur", "Value"), bi("Source", "Source")],
        limits.values.map(([label, value, source]) => [
          text(label),
          `<td class="key strong">${escape(value)}</td>`,
          muted(source),
        ]),
        ["42%", "22%", "36%"],
      ),
    )}
    ${card(
      bi("Plafonds de corps HTTP", "HTTP body ceilings"),
      "server/src/lib.rs",
      table(
        [bi("Route", "Route"), bi("Plafond", "Ceiling")],
        limits.bodyLimits.map((entry) => [key(entry.route, true), muted(entry.limit)]),
        ["62%", "38%"],
      ),
    )}
    ${card(
      bi("Types MIME admis", "Accepted MIME types"),
      "allowed_media_type()",
      table(
        [bi("Type", "Type")],
        limits.mediaTypes.map((type) => [key(type, type === "application/octet-stream")]),
        ["100%"],
      ) +
        note(
          "application/octet-stream est le joker : n’importe quel fichier passe sous cette étiquette. La liste borne ce qui s’affiche en ligne, elle ne certifie pas qu’un fichier est sûr — c’est ce que dit l’ADR 0003.",
          "application/octet-stream is the wildcard: any file passes under that label. The list bounds what renders inline, it does not certify a file is safe — as ADR 0003 states.",
        ),
    )}
  </div>
</section>`,
  });
}

export function renderData(migrations, states) {
  return page({
    slug: "data",
    titleFr: "Données et cycles de vie",
    titleEn: "Data and lifecycles",
    introFr:
      "Le registre des migrations avec la phase que chacune déclare, et les états que le domaine reconnaît. La politique est forward-only : une migration livrée n’est jamais réécrite.",
    introEn:
      "The migration ledger with the phase each one declares, and the states the domain recognises. The policy is forward-only: a shipped migration is never rewritten.",
    body: `
${summary([
  [migrations.length, "migrations", "migrations"],
  [migrations.filter((entry) => entry.phase === "expand").length, "phase expand", "expand phase"],
  [states.incidentStatuses.length, "états incident", "incident states"],
  [states.severities.length, "sévérités", "severities"],
  [states.releaseStates.length, "états release", "release states"],
])}
<section>
  <div class="section-head"><h2>${bi("Cycles de vie", "Lifecycles")}</h2></div>
  <div class="capture-grid">
    ${card(
      bi("Incident", "Incident"),
      "server/src/domain/incident.rs",
      table(
        [bi("Statut", "Status")],
        states.incidentStatuses.map((status) => [key(status, true)]),
        ["100%"],
      ),
    )}
    ${card(
      bi("Sévérité", "Severity"),
      "server/src/domain/incident.rs",
      table(
        [bi("Niveau", "Level")],
        states.severities.map((severity) => [key(severity, true)]),
        ["100%"],
      ),
    )}
    ${card(
      bi("Release", "Release"),
      "server/src/domain/release.rs",
      table(
        [bi("État", "State")],
        states.releaseStates.map((state) => [key(state, true)]),
        ["100%"],
      ),
      true,
    )}
  </div>
</section>
<section>
  <div class="section-head"><h2>${bi("Registre des migrations", "Migration ledger")}</h2></div>
  <div class="capture-grid single">
    ${card(
      bi("server/migrations", "server/migrations"),
      `${migrations.length} ${migrations.length > 1 ? "fichiers" : "fichier"}`,
      table(
        [
          bi("N°", "No."),
          bi("Fichier", "File"),
          bi("Phase", "Phase"),
          bi("Instructions", "Statements"),
          bi("Intention", "Intent"),
        ],
        migrations.map((entry) => [
          key(entry.number, true),
          key(entry.file.replace(/^\d+_/, "")),
          cell(entry.phase ? tone("ghost", escape(entry.phase)) : `<span class="muted">—</span>`),
          text(entry.statements),
          muted(entry.summary ? entry.summary.slice(0, 120) : null),
        ]),
        ["6%", "26%", "11%", "11%", "46%"],
      ) +
        note(
          "Le numéro 0018 n’existe pas : un trou dans la séquence est sans conséquence pour sqlx, qui ordonne par numéro et non par continuité.",
          "Number 0018 does not exist: a gap in the sequence is harmless to sqlx, which orders by number rather than by continuity.",
        ),
      true,
    )}
  </div>
</section>`,
  });
}

export function renderAutomations(catalog, api) {
  if (!catalog.available) {
    return page({
      slug: "automations",
      titleFr: "Automations",
      titleEn: "Automations",
      introFr: "Cette planche est servie par le serveur en fonctionnement.",
      introEn: "This board is served by the running server.",
      body: `<section><div class="capture-grid single">${card(
        bi("Serveur injoignable", "Server unreachable"),
        api,
        note(
          `Aucun catalogue récupéré : ${catalog.reason}. Lancez la pile (just up) puis régénérez.`,
          `No catalogue retrieved: ${catalog.reason}. Start the stack (just up) then regenerate.`,
        ),
        true,
      )}</div></section>`,
    });
  }

  const actions = catalog.services.reduce((total, service) => total + service.actions.length, 0);
  const reactions = catalog.services.reduce(
    (total, service) => total + service.reactions.length,
    0,
  );

  return page({
    slug: "automations",
    titleFr: "Automations",
    titleEn: "Automations",
    introFr:
      "Les services intégrés, avec ce que chacun peut déclencher (action) et ce qu’il peut exécuter en retour (réaction). Lu depuis /about.json du serveur en fonctionnement, donc exactement ce que voit un client.",
    introEn:
      "Integrated services, with what each can trigger (action) and what it can run in return (reaction). Read from the running server’s /about.json, so exactly what a client sees.",
    body: `
${summary([
  [catalog.services.length, "services", "services"],
  [actions, "actions", "actions"],
  [reactions, "réactions", "reactions"],
])}
<section>
  <div class="section-head"><h2>${bi("Par service", "By service")}</h2>
  <p>${bi("Source live : /about.json", "Live source: /about.json")}</p></div>
  <div class="capture-grid">
    ${catalog.services
      .map((service) =>
        card(
          escape(service.label ?? service.name),
          service.name,
          table(
            [bi("Type", "Kind"), bi("Nom", "Name"), bi("Intention", "Intent")],
            [
              ...service.actions.map((action) => [
                cell(tone("info", bi("action", "action"))),
                key(action.name, true),
                muted(action.description ?? action.label),
              ]),
              ...service.reactions.map((reaction) => [
                cell(tone("success", bi("réaction", "reaction"))),
                key(reaction.name, true),
                muted(reaction.description ?? reaction.label),
              ]),
            ],
            ["18%", "30%", "52%"],
          ),
        ),
      )
      .join("")}
  </div>
</section>`,
  });
}

export function renderUi(primitives, tokenData) {
  return page({
    slug: "ui",
    titleFr: "Primitives et jetons",
    titleEn: "Primitives and tokens",
    introFr:
      "Les composants de components/ui et les axes que chacun accepte, plus les jetons de design déclarés sur :root. C’est le socle que toutes les planches produit réutilisent.",
    introEn:
      "The components in components/ui and the axes each accepts, plus the design tokens declared on :root. This is the base every product board reuses.",
    body: `
${summary([
  [primitives.length, "primitives", "primitives"],
  [primitives.filter((primitive) => primitive.hasTest).length, "avec test", "with a test"],
  [
    primitives.reduce(
      (total, primitive) =>
        total + primitive.axes.reduce((sum, axis) => sum + axis.values.length, 0),
      0,
    ),
    "variantes",
    "variants",
  ],
  [tokenData.total, "jetons", "tokens"],
  [tokenData.families.length, "familles de jetons", "token families"],
])}
<section>
  <div class="section-head"><h2>${bi("Primitives d’interface", "Interface primitives")}</h2></div>
  <div class="capture-grid single">
    ${card(
      bi("components/ui", "components/ui"),
      `client-web/components/ui — ${primitives.length}`,
      table(
        [
          bi("Composant", "Component"),
          bi("Axes", "Axes"),
          bi("Test", "Test"),
          bi("Lignes", "Lines"),
        ],
        primitives.map((primitive) => [
          key(primitive.component, true),
          cell(
            primitive.axes.length > 0
              ? primitive.axes
                  .map(
                    (axis) =>
                      `<span class="muted">${escape(axis.axis)}</span> ${axis.values
                        .map((value) => tone("ghost", escape(value)))
                        .join(" ")}`,
                  )
                  .join("<br />")
              : `<span class="muted">—</span>`,
          ),
          cell(yesNo(primitive.hasTest)),
          muted(primitive.lines),
        ]),
        ["19%", "58%", "9%", "14%"],
      ),
      true,
    )}
  </div>
</section>
<section>
  <div class="section-head"><h2>${bi("Jetons de design", "Design tokens")}</h2>
  <p>${bi("Déclarés sur :root dans globals.css", "Declared on :root in globals.css")}</p></div>
  <div class="capture-grid">
    ${tokenData.families
      .map((family) =>
        card(
          escape(family.family),
          `${family.items.length}`,
          table(
            [bi("Jeton", "Token"), bi("Valeur", "Value")],
            family.items.map((token) => [
              key(`--${token.name}`, true),
              cell(
                `${/^(#|rgb|hsl|oklch)/i.test(token.value) ? `<span class="tokenchip" style="background:${escape(token.value)}"></span>` : ""}<span class="muted">${escape(token.value)}</span>`,
              ),
            ]),
            ["52%", "48%"],
          ),
        ),
      )
      .join("")}
  </div>
</section>`,
  });
}
