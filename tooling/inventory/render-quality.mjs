import { bi, card, note, page, summary, table, tone, yesNo } from "./layout.mjs";
import { cell, key, muted, text } from "./render-helpers.mjs";

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
          bi("Marge EN", "EN slack"),
          bi("Mots FR", "FR words"),
          bi("Marge FR", "FR slack"),
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
                  ? tone("warning", "0")
                  : tone("success", String(entry.locales.en.slack)),
            ),
            text(entry.locales.fr.words),
            cell(
              entry.locales.fr.ceiling === null
                ? `<span class="muted">—</span>`
                : entry.locales.fr.slack === 0
                  ? tone("warning", "0")
                  : tone("success", String(entry.locales.fr.slack)),
            ),
            muted(`${ratio >= 0 ? "+" : ""}${ratio}%`),
          ];
        }),
        ["18%", "8%", "12%", "12%", "12%", "12%", "13%"],
      ) +
        note(
          "La marge est le nombre de mots qu’un namespace peut encore gagner. À zéro, le plafond a été re-mesuré sur l’état exact du jour : plus un mot n’entre sans faire tomber le test. Une marge positive est du mou disponible.",
          "Slack is how many words a namespace can still gain. At zero, the ceiling was re-measured against the exact state of the day: not one more word fits without failing the test. Positive slack is give still available.",
        ),
      true,
    )}
  </div>
</section>`,
  });
}

// Les enfants de la section Product dans la nav. Un hub liste ses enfants,
// ni plus ni moins : chaque autre planche est listée par sa propre section.
const PRODUCT_BOARDS = [
  { slug: "capabilities", fr: "Rôles et permissions", en: "Roles and permissions" },
  { slug: "conversations", fr: "Conversations", en: "Conversations" },
  { slug: "automations", fr: "Automations", en: "Automations" },
];

export function renderIndex(stats) {
  return page({
    slug: "index",
    titleFr: "Produit",
    titleEn: "Product",
    introFr:
      "Ce que le produit autorise, ce que ses conversations savent faire et ce que ses automations déclenchent. Ces trois planches sont générées depuis le code.",
    introEn:
      "What the product authorises, what its conversations can do and what its automations trigger. These three boards are generated from the source.",
    body: `
<div class="grid cards">
  <ul>
    ${PRODUCT_BOARDS.map((board) => {
      const stat = stats.find((entry) => entry.slug === board.slug);
      return `<li><p><strong><a href="${board.slug}.html">${bi(board.fr, board.en)}</a></strong></p>
<p>${bi(stat.subFr, stat.subEn)}</p></li>`;
    }).join("")}
  </ul>
</div>`,
  });
}

// --- build -----------------------------------------------------------------
