// --- tooling/inventory/layout.mjs ---
//
// Shared shell for every inventory page. The palette mirrors the product's own
// dark tokens so a capture from here sits next to a product screenshot without
// looking foreign.

export const PAGES = [
  { slug: "index", fr: "Vue d’ensemble", en: "Overview" },
  { slug: "capabilities", fr: "Rôles", en: "Roles" },
  { slug: "errors", fr: "Erreurs", en: "Errors" },
  { slug: "api", fr: "API", en: "API" },
  { slug: "events", fr: "Événements", en: "Events" },
  { slug: "conversations", fr: "Conversations", en: "Conversations" },
  { slug: "data", fr: "Données", en: "Data" },
  { slug: "automations", fr: "Automations", en: "Automations" },
  { slug: "ui", fr: "Interface", en: "Interface" },
  { slug: "contracts", fr: "Contrats DOM", en: "DOM contracts" },
  { slug: "i18n", fr: "Textes", en: "Copy" },
  { slug: "badges", fr: "Badges", en: "Badges" },
];

export function escape(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
    (character) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character],
  );
}

/** Bilingual text node, toggled client-side like the badge inventory. */
export function bi(fr, en) {
  return `<span data-fr="${escape(fr)}" data-en="${escape(en)}">${escape(fr)}</span>`;
}

export function tone(kind, label) {
  return `<span class="pill ${kind}">${label}</span>`;
}

export function yesNo(value) {
  return value
    ? `<span class="mark yes" title="yes">&#10003;</span>`
    : `<span class="mark no" title="no">&#8212;</span>`;
}

export function summary(items) {
  return `<div class="summary">${items
    .map(
      ([value, fr, en]) => `<div><strong>${escape(value)}</strong><span>${bi(fr, en)}</span></div>`,
    )
    .join("")}</div>`;
}

export function card(title, source, inner, wide = false) {
  return `<article class="capture-card${wide ? " wide" : ""}">
  <div class="card-head"><h3>${title}</h3>${source ? `<code>${escape(source)}</code>` : ""}</div>
  ${inner}
</article>`;
}

export function table(columns, rows, widths) {
  const colgroup = widths
    ? `<colgroup>${widths.map((w) => `<col style="width:${w}" />`).join("")}</colgroup>`
    : "";
  return `<div class="table-wrap"><table>${colgroup}
<thead><tr>${columns.map((column) => `<th>${column}</th>`).join("")}</tr></thead>
<tbody>${rows.map((row) => `<tr>${row.map((cell) => cell ?? "").join("")}</tr>`).join("")}</tbody>
</table></div>`;
}

export function note(fr, en) {
  return `<p class="note">${bi(fr, en)}</p>`;
}

const CSS = `
:root{color-scheme:dark;--bg:#15161a;--panel:#1c1d22;--panel-2:#25262d;--border:rgb(255 255 255/8%);
--text:#e7e7ea;--muted:#989ba1;--muted-2:#878b93;--gold:#fbc02d;--neutral:#57606a;--info:#0969da;
--warning:#9a6700;--danger:#cf222e;--success:#1a7f37;font-family:Inter,ui-sans-serif,system-ui,sans-serif}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text)}
main{width:min(1280px,calc(100% - 32px));margin:0 auto;padding:32px 0 72px}
h1,h2,h3,p{margin:0}
h1{font-size:clamp(22px,3vw,32px);letter-spacing:-.035em}
h2{font-size:15px}
h3{font-size:12px}
a{color:inherit}
nav.portal{display:flex;align-items:center;justify-content:space-between;gap:20px;margin-bottom:30px;
padding-bottom:14px;border-bottom:1px solid var(--border)}
nav.portal>div{display:flex;flex-wrap:wrap;justify-content:flex-end;gap:4px}
nav.portal a{padding:6px 9px;border-radius:6px;color:var(--muted);font-size:12px;font-weight:600;
text-decoration:none}
nav.portal a:hover{background:var(--panel);color:var(--text)}
nav.portal a.brand{padding-left:0;color:var(--text);font-size:14px}
nav.portal a[aria-current=page]{background:var(--panel-2);color:var(--text)}
header.top{display:flex;align-items:end;justify-content:space-between;gap:24px;margin-bottom:18px}
.intro{max-width:820px;margin-top:9px;color:var(--muted);font-size:13.5px;line-height:1.55}
.locale{display:inline-flex;padding:3px;border:1px solid var(--border);border-radius:7px;background:var(--panel)}
.locale button{border:0;border-radius:5px;padding:7px 11px;background:transparent;color:var(--muted);
cursor:pointer;font:inherit;font-size:12px;font-weight:650}
.locale button[aria-pressed=true]{background:var(--panel-2);color:var(--text)}
nav.pages{display:flex;flex-wrap:wrap;gap:4px;margin-bottom:22px;padding-bottom:16px;border-bottom:1px solid var(--border)}
nav.pages a{padding:6px 10px;border:1px solid transparent;border-radius:6px;color:var(--muted);
font-size:12px;font-weight:600;text-decoration:none}
nav.pages a:hover{background:var(--panel)}
nav.pages a[aria-current=page]{border-color:var(--border);background:var(--panel-2);color:var(--text)}
.summary{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));overflow:hidden;
border:1px solid var(--border);border-radius:8px;background:var(--panel)}
.summary div{padding:13px 16px;border-right:1px solid var(--border)}
.summary div:last-child{border-right:0}
.summary strong{display:block;font-size:19px}
.summary span{color:var(--muted);font-size:10px;letter-spacing:.08em;text-transform:uppercase}
section{margin-top:30px}
.section-head{display:flex;align-items:baseline;justify-content:space-between;gap:16px;margin-bottom:10px}
.section-head p{color:var(--muted);font-size:12px}
.capture-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;align-items:start}
.capture-grid.single{grid-template-columns:minmax(0,1fr)}
.capture-card{overflow:hidden;border:1px solid var(--border);border-radius:8px;background:var(--panel);
break-inside:avoid;isolation:isolate}
.capture-card.wide{grid-column:1/-1}
.card-head{display:flex;align-items:baseline;justify-content:space-between;gap:16px;min-height:43px;
padding:12px 14px;border-bottom:1px solid var(--border)}
.card-head code{color:var(--muted-2);font:10px ui-monospace,SFMono-Regular,Menlo,monospace}
.table-wrap{overflow-x:auto}
table{width:100%;border-collapse:collapse;font-size:12px}
th{position:sticky;top:0;padding:8px 14px;background:var(--panel-2);color:var(--muted);text-align:left;
font-size:9px;font-weight:700;letter-spacing:.055em;text-transform:uppercase;white-space:nowrap}
td{padding:9px 14px;border-top:1px solid var(--border);vertical-align:middle}
tbody tr:first-child td{border-top:0}
tbody tr:hover td{background:rgb(255 255 255/2%)}
.key{overflow-wrap:anywhere;color:var(--muted);font:10.5px ui-monospace,SFMono-Regular,Menlo,monospace}
.key.strong{color:var(--text)}
.muted{color:var(--muted-2);font-size:11px}
.note{margin:0;padding:10px 14px;border-top:1px solid var(--border);color:var(--muted-2);
font-size:11px;line-height:1.5}
.pill{display:inline-flex;align-items:center;height:20px;padding:0 7px;border-radius:4px;
color:#fff;font-size:11px;font-weight:650;line-height:1;white-space:nowrap}
.pill.neutral{background:var(--neutral)}.pill.info{background:var(--info)}
.pill.warning{background:var(--warning)}.pill.danger{background:var(--danger)}
.pill.success{background:var(--success)}
.pill.ghost{background:transparent;border:1px solid var(--border);color:var(--muted)}
.mark{font-size:13px;font-weight:700}
.mark.yes{color:#3fb950}.mark.no{color:var(--muted-2)}
.grid-cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:12px}
.tokenchip{display:inline-block;width:14px;height:14px;border:1px solid var(--border);border-radius:3px;
vertical-align:-3px;margin-right:6px}
.bar{position:relative;height:6px;border-radius:3px;background:var(--panel-2);overflow:hidden;min-width:70px}
.bar i{position:absolute;inset:0 auto 0 0;background:var(--gold);border-radius:3px}
.bar.over i{background:var(--danger)}
.legend{display:flex;flex-wrap:wrap;gap:14px;margin-top:10px;color:var(--muted-2);font-size:11px}
.hub{display:grid;grid-template-columns:repeat(auto-fill,minmax(268px,1fr));gap:12px}
.hub a{display:block;padding:16px;border:1px solid var(--border);border-radius:8px;background:var(--panel);
text-decoration:none}
.hub a:hover{border-color:rgb(255 255 255/18%);background:var(--panel-2)}
.hub strong{display:block;font-size:26px;letter-spacing:-.02em}
.hub .label{display:block;margin-top:2px;font-size:12.5px;font-weight:650}
.hub .sub{display:block;margin-top:6px;color:var(--muted-2);font-size:11px;line-height:1.45}
@media(max-width:900px){.capture-grid{grid-template-columns:minmax(0,1fr)}}
@media(max-width:700px){nav.portal{align-items:flex-start;flex-direction:column}nav.portal>div{justify-content:flex-start}}
`;

const SCRIPT = `
(function(){
  var stored=null;
  try{stored=localStorage.getItem("opswarden-inventory-locale")}catch(e){}
  var locale=stored==="en"||stored==="fr"?stored:"fr";
  function apply(next){
    locale=next;
    document.documentElement.lang=next;
    document.querySelectorAll("[data-fr]").forEach(function(node){
      node.textContent=node.getAttribute("data-"+next)||node.getAttribute("data-fr");
    });
    document.querySelectorAll(".locale button").forEach(function(button){
      button.setAttribute("aria-pressed",String(button.dataset.locale===next));
    });
    try{localStorage.setItem("opswarden-inventory-locale",next)}catch(e){}
  }
  document.addEventListener("click",function(event){
    var button=event.target.closest(".locale button");
    if(button)apply(button.dataset.locale);
  });
  apply(locale);
})();
`;

export function page({ slug, titleFr, titleEn, introFr, introEn, body }) {
  const nav = PAGES.map(
    (entry) =>
      `<a href="${entry.slug}.html"${entry.slug === slug ? ' aria-current="page"' : ""}>${bi(entry.fr, entry.en)}</a>`,
  ).join("");

  return `<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>OpsWarden — ${escape(titleFr)}</title>
<style>${CSS}</style>
</head>
<body>
<main>
  <nav class="portal" aria-label="Documentation">
    <a class="brand" href="https://opswarden-git.github.io/opswarden/">OpsWarden</a>
    <div>
      <a href="index.html" aria-current="page">${bi("Inventaire", "Inventory")}</a>
      <a href="https://opswarden-git.github.io/opswarden/getting-started/">${bi("Démarrer", "Run")}</a>
      <a href="https://opswarden-git.github.io/opswarden/architecture/">Architecture</a>
      <a href="https://opswarden-git.github.io/opswarden/reference/">Référence</a>
      <a href="https://opswarden-git.github.io/opswarden/design/">Design</a>
      <a href="https://opswarden-git.github.io/opswarden/contributing/">${bi("Contribuer", "Contribute")}</a>
    </div>
  </nav>
  <header class="top">
    <div>
      <h1>${bi(titleFr, titleEn)}</h1>
      <p class="intro">${bi(introFr, introEn)}</p>
    </div>
    <div class="locale">
      <button type="button" data-locale="fr" aria-pressed="true">FR</button>
      <button type="button" data-locale="en" aria-pressed="false">EN</button>
    </div>
  </header>
  <nav class="pages">${nav}</nav>
  ${body}
</main>
<script>${SCRIPT}</script>
</body>
</html>`;
}
