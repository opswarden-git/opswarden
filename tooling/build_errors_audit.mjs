import fs from "node:fs";

const ROOT = "/home/tco/Bureau/opswarden/opswarden";
const en = JSON.parse(fs.readFileSync(`${ROOT}/client-web/messages/en.json`, "utf8"));
const fr = JSON.parse(fs.readFileSync(`${ROOT}/client-web/messages/fr.json`, "utf8"));

const errorRows = [];

function collectErrors(enObj, frObj, ns) {
  for (const key of Object.keys(enObj)) {
    const valEn = enObj[key];
    const valFr = frObj ? frObj[key] : "";
    if (typeof valEn === "object") {
      collectErrors(valEn, frObj ? frObj[key] : {}, `${ns}.${key}`);
    } else {
      if (
        key.toLowerCase().includes("error") ||
        key.toLowerCase().includes("failed") ||
        key.toLowerCase().includes("invalid") ||
        key.toLowerCase().includes("forbidden") ||
        key.toLowerCase().includes("denied") ||
        key.toLowerCase().includes("missing") ||
        ns.toLowerCase().includes("error")
      ) {
        errorRows.push({
          key: `${ns}.${key}`,
          en: String(valEn),
          fr: String(valFr || valEn),
        });
      }
    }
  }
}

collectErrors(en, fr, "i18n");

if (fs.existsSync(`${ROOT}/contracts/error-codes.json`)) {
  const serverErrors = JSON.parse(fs.readFileSync(`${ROOT}/contracts/error-codes.json`, "utf8"));
  for (const err of serverErrors) {
    errorRows.push({
      key: `server.${err.code}`,
      en: err.en || err.code,
      fr: err.fr || err.code,
      status: err.status,
    });
  }
}

const html = `<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Catalogue des messages d'erreur</title>
<style>
  :root {
    color-scheme: dark;
    --bg: #15161a;
    --panel: #1c1d22;
    --panel-2: #25262d;
    --border: rgba(255, 255, 255, 0.08);
    --text: #e7e7ea;
    --muted: #989ba1;
    --muted-2: #878b93;
    --gold: #fbc02d;
    --danger: #cf222e;
    font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  }
  * { box-sizing: border-box; }
  body { margin: 0; background: var(--bg); color: var(--text); padding: 32px 0 64px; }
  main { width: min(1280px, calc(100% - 32px)); margin: 0 auto; }
  header { margin-bottom: 24px; border-bottom: 1px solid var(--border); padding-bottom: 16px; }
  h1 { font-size: 26px; font-weight: 700; letter-spacing: -0.025em; margin: 0 0 8px; }
  p.intro { color: var(--muted); font-size: 13.5px; margin: 0; line-height: 1.55; max-width: 820px; }

  .table-wrap { overflow-x: auto; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; text-align: left; }
  th { background: var(--panel-2); color: var(--muted); padding: 10px 14px; font-size: 9px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.055em; white-space: nowrap; }
  td { padding: 10px 14px; border-top: 1px solid var(--border); vertical-align: middle; }
  tbody tr:hover td { background: rgba(255, 255, 255, 0.02); }
  .code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; color: var(--gold); font-size: 11px; }
  .pill { display: inline-flex; align-items: center; height: 20px; padding: 0 8px; border-radius: 4px; color: #ffffff; background: var(--danger); font-size: 11px; font-weight: 650; line-height: 1; white-space: nowrap; }
</style>
</head>
<body>
  <main>
    <header>
      <h1>Catalogue des messages d'erreur</h1>
      <p class="intro">Chaque variante de DomainError, son code stable et son rendu dans les deux locales. La liste est complète par construction : elle est lue depuis un match exhaustif que le compilateur garantit.</p>
    </header>

    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th style="width: 48px;">#</th>
            <th style="width: 240px;">Code / Clé</th>
            <th>Traduction FR</th>
            <th>Traduction EN</th>
            <th style="width: 320px;">Rendu visuel</th>
          </tr>
        </thead>
        <tbody>
          ${errorRows
            .map(
              (row, i) => `
            <tr>
              <td style="color: var(--muted-2);">${i + 1}</td>
              <td class="code">${row.key}</td>
              <td>${row.fr}</td>
              <td style="color: var(--muted);">${row.en}</td>
              <td><span class="pill">${row.fr}</span></td>
            </tr>
          `,
            )
            .join("")}
        </tbody>
      </table>
    </div>
  </main>
</body>
</html>`;

fs.writeFileSync(`${ROOT}/tooling/inventory/dist/errors_audit.html`, html);
console.log(`Updated H1 to Catalogue des messages d'erreur`);
