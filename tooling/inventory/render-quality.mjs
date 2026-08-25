import { bi, card, escape, note, page, summary, table, tone, yesNo } from "./layout.mjs";
import { cell, key, muted, text, STATUS_TONE, METHOD_TONE } from "./render-helpers.mjs";

export function renderContracts(attributes) {
  const uncovered = attributes.filter((attribute) => !attribute.covered);
  return page({
    slug: "contracts",
    titleFr: "Contrats DOM",
    titleEn: "DOM contracts",
    introFr:
      "Les attributs data-* que les composants déclarent, et lesquels la suite navigateur sélectionne réellement. Un attribut déclaré que nulle spec ne lit est soit du poids mort, soit une garantie non testée.",
    introEn:
      "The data-* attributes components declare, and which ones the browser suite actually selects. A declared attribute no spec reads is either dead weight or an untested guarantee.",
    body: `
${summary([
  [attributes.length, "attributs déclarés", "declared attributes"],
  [attributes.length - uncovered.length, "lus par les E2E", "read by the E2E suite"],
  [uncovered.length, "jamais sélectionnés", "never selected"],
  [
    `${Math.round(((attributes.length - uncovered.length) / attributes.length) * 100)}%`,
    "de couverture",
    "coverage",
  ],
])}
<section>
  <div class="section-head"><h2>${bi("Tous les attributs", "Every attribute")}</h2>
  <p>${bi("Non couverts en premier", "Uncovered first")}</p></div>
  <div class="capture-grid single">
    ${card(
      bi("data-*", "data-*"),
      "client-web/ · tooling/e2e/",
      table(
        [bi("Attribut", "Attribute"), bi("E2E", "E2E"), bi("Déclaré dans", "Declared in")],
        attributes.map((attribute) => [
          key(attribute.attribute, true),
          cell(yesNo(attribute.covered)),
          muted(
            attribute.files
              .map((file) => file.replace("client-web/", ""))
              .slice(0, 2)
              .join(", ") + (attribute.files.length > 2 ? ` +${attribute.files.length - 2}` : ""),
          ),
        ]),
        ["30%", "10%", "60%"],
      ) +
        note(
          "Un attribut non couvert n’est pas un défaut en soi : certains servent au style ou au débogage. Le chiffre utile est la tendance — il ne devrait pas monter sans raison.",
          "An uncovered attribute is not a defect in itself: some serve styling or debugging. The useful number is the trend — it should not climb without a reason.",
        ),
      true,
    )}
  </div>
</section>`,
  });
}

export function renderI18n(data) {
  return page({
    slug: "i18n",
    titleFr: "Budget de texte",
    titleEn: "Copy budget",
    introFr:
      "Combien de prose chaque écran porte, par namespace et par locale, face au plafond que le test de ratchet impose. Un plafond ne peut que descendre ; le relever doit être un geste délibéré.",
    introEn:
      "How much prose each screen carries, per namespace and per locale, against the ceiling the ratchet test enforces. A ceiling can only go down; raising one must be a deliberate act.",
    body: `
${summary([
  [data.namespaces.length, "namespaces", "namespaces"],
  [data.totalKeys, "clés par locale", "keys per locale"],
  [
    data.namespaces.reduce((total, entry) => total + entry.locales.en.words, 0),
    "mots EN",
    "EN words",
  ],
  [
    data.namespaces.reduce((total, entry) => total + entry.locales.fr.words, 0),
    "mots FR",
    "FR words",
  ],
  [data.namespaces.filter((entry) => entry.missingInFr !== 0).length, "écarts de clés", "key gaps"],
])}
<section>
  <div class="section-head"><h2>${bi("Par namespace", "Per namespace")}</h2>
  <p>${bi("Marge = plafond moins mesure", "Slack = ceiling minus measurement")}</p></div>
  <div class="capture-grid single">
    ${card(
      bi("text-budget.test.ts", "text-budget.test.ts"),
      "client-web/i18n/",
      table(
        [
          bi("Namespace", "Namespace"),
          bi("Clés", "Keys"),
          bi("Mots EN", "EN words"),
          bi("Marge", "Slack"),
          bi("Mots FR", "FR words"),
          bi("Marge", "Slack"),
          bi("Écart FR/EN", "FR/EN gap"),
        ],
        data.namespaces.map((entry) => {
          const ratio =
            entry.locales.en.words > 0
              ? Math.round((entry.locales.fr.words / entry.locales.en.words - 1) * 100)
              : 0;
          return [
            key(entry.namespace, true),
            muted(entry.keys),
            text(entry.locales.en.words),
            cell(
              entry.locales.en.ceiling === null
                ? `<span class="muted">—</span>`
                : entry.locales.en.slack === 0
                  ? tone("success", "0")
                  : tone("warning", String(entry.locales.en.slack)),
            ),
            text(entry.locales.fr.words),
            cell(
              entry.locales.fr.ceiling === null
                ? `<span class="muted">—</span>`
                : entry.locales.fr.slack === 0
                  ? tone("success", "0")
                  : tone("warning", String(entry.locales.fr.slack)),
            ),
            muted(`${ratio >= 0 ? "+" : ""}${ratio}%`),
          ];
        }),
        ["18%", "8%", "12%", "12%", "12%", "12%", "13%"],
      ) +
        note(
          "Une marge à zéro veut dire que le plafond a été re-mesuré sur l’état exact du jour. Une marge positive est du mou : la prose peut regrandir jusque-là sans qu’aucun test ne bronche.",
          "Zero slack means the ceiling was re-measured against the exact state of the day. Positive slack is give: prose can regrow up to it without any test complaining.",
        ),
      true,
    )}
  </div>
</section>`,
  });
}

export function renderIndex(stats) {
  return page({
    slug: "index",
    titleFr: "Inventaire OpsWarden",
    titleEn: "OpsWarden inventory",
    introFr:
      "Onze familles d’ensembles fermés, dérivées du code plutôt que transcrites. Chaque planche est lue depuis un match exhaustif, un contrat vérifié par un test, ou le serveur en fonctionnement — donc aucune ne peut diverger en silence de ce qu’elle documente.",
    introEn:
      "Eleven families of closed sets, derived from the code rather than transcribed. Every board is read from an exhaustive match, a contract asserted by a test, or the running server — so none can quietly drift from what it documents.",
    body: `
<section>
  <div class="section-head"><h2>${bi("Planches", "Boards")}</h2>
  <p>${bi("Régénérées par just inventory", "Regenerated by just inventory")}</p></div>
  <div class="hub">
    ${stats
      .map(
        (entry) =>
          `<a href="${entry.slug}.html"><strong>${escape(entry.count)}</strong>
<span class="label">${bi(entry.fr, entry.en)}</span>
<span class="sub">${bi(entry.subFr, entry.subEn)}</span></a>`,
      )
      .join("")}
  </div>
</section>`,
  });
}

// --- build -----------------------------------------------------------------
