import { bi, card, escape, note, page, summary, table, tone, yesNo } from "./layout.mjs";
import { cell, key, muted, text, statusTone, statusLabel, METHOD_TONE } from "./render-helpers.mjs";

export function renderCapabilities(data) {
  const rows = data.fields.map((field) => [
    key(field.field, true),
    ...data.roles.map((role) => cell(yesNo(field.byRole[role]))),
  ]);
  const totals = data.roles.map((role) => data.fields.filter((field) => field.byRole[role]).length);

  return page({
    slug: "capabilities",
    titleFr: "Rôles et permissions",
    titleEn: "Roles and permissions",
    introFr:
      "Les 17 capacités produit dérivées d’une appartenance à une équipe. Le serveur reste l’autorité ; le client web ne fait que refléter ce contrat pour éviter d’afficher une action qui sera refusée. La grille est lue depuis contracts/role-capabilities.json, qu’un test Rust vérifie contre le serveur.",
    introEn:
      "The 17 product capabilities derived from one team membership. The server remains the authority; the web client mirrors this contract only to avoid rendering an action it would reject. The grid is read from contracts/role-capabilities.json, which a Rust test asserts against the server.",
    body: `
${summary([
  [data.roles.length, "rôles", "roles"],
  [data.fields.length, "capacités", "capabilities"],
  [data.roles.length * data.fields.length, "cellules", "cells"],
  ...data.roles.map((role, index) => [totals[index], `accordées à ${role}`, `granted to ${role}`]),
])}
<section>
  <div class="section-head"><h2>${bi("Grille complète", "Full grid")}</h2></div>
  <div class="capture-grid single">
    ${card(
      bi("derive_capabilities()", "derive_capabilities()"),
      "server/src/domain/capabilities.rs",
      table([bi("Capacité", "Capability"), ...data.roles.map((role) => escape(role))], rows, [
        "46%",
        "18%",
        "18%",
        "18%",
      ]) +
        note(
          "Une capacité vraie pour les trois rôles n’est pas une permission : c’est un comportement de base. Deux le sont ici — réagir sur la timeline et envoyer un message privé.",
          "A capability true for all three roles is not a permission: it is baseline behaviour. Two are — reacting on the timeline and sending a direct message.",
        ),
      true,
    )}
  </div>
</section>`,
  });
}

export function renderErrors(data) {
  const rows = data.rows.map((row) => [
    key(row.code, true),
    cell(tone(statusTone(row.status), escape(statusLabel(row.status)))),
    text(row.en),
    text(row.fr),
  ]);
  const byStatus = new Map();
  for (const row of data.rows) byStatus.set(row.status, (byStatus.get(row.status) ?? 0) + 1);

  const untranslated = data.rows.filter((row) => !row.en || !row.fr);

  return page({
    slug: "errors",
    titleFr: "Catalogue d’erreurs",
    titleEn: "Error catalogue",
    introFr:
      "Chaque variante de DomainError, son code stable, le statut HTTP qu’elle produit et son rendu dans les deux locales. La liste est complète par construction : elle est lue depuis un match exhaustif que le compilateur garantit.",
    introEn:
      "Every DomainError variant, its stable code, the HTTP status it produces and how it reads in both locales. The list is complete by construction: it is read from an exhaustive match the compiler guarantees.",
    body: `
${summary([
  [data.rows.length, "erreurs serveur", "server errors"],
  [byStatus.size, "statuts HTTP", "HTTP statuses"],
  [data.clientCodes.length, "codes client", "client codes"],
  [untranslated.length, "sans traduction", "untranslated"],
  [data.orphanKeys.length, "clés orphelines", "orphan keys"],
])}
<section>
  <div class="section-head"><h2>${bi("Répartition par statut", "Split by status")}</h2></div>
  <div class="capture-grid single">
    ${card(
      bi("into_response()", "into_response()"),
      "server/src/handlers/error.rs",
      table(
        [bi("Statut", "Status"), bi("Erreurs", "Errors"), ""],
        [...byStatus.entries()]
          .sort((a, b) => b[1] - a[1])
          .map(([status, count]) => [
            cell(
              tone(
                statusTone(status),
                escape(statusLabel(status)),
              ),
            ),
            text(count),
            cell(
              `<div class="bar"><i style="width:${Math.round((count / data.rows.length) * 100)}%"></i></div>`,
            ),
          ]),
        ["26%", "12%", "62%"],
      ),
      true,
    )}
  </div>
</section>
${
  untranslated.length > 0
    ? `<section>
  <div class="section-head"><h2>${bi("Codes sans message d’interface", "Codes with no interface message")}</h2>
  <p>${bi("Le client retombe sur un libellé générique", "The client falls back to a generic label")}</p></div>
  <div class="capture-grid single">
    ${card(
      bi("Non traduits", "Untranslated"),
      "client-web/messages/{en,fr}.json",
      table(
        [bi("Code", "Code"), bi("Statut", "Status"), bi("Intention", "Intent")],
        untranslated.map((row) => [
          key(row.code, true),
          cell(
            tone(
              statusTone(row.status),
              escape(statusLabel(row.status)),
            ),
          ),
          muted(row.doc),
        ]),
        ["34%", "18%", "48%"],
      ) +
        note(
          "La plupart sont des échecs internes d’automation ou d’e-mail qu’un opérateur ne voit jamais. too_many_attempts est l’exception : c’est le 429 du throttle de connexion, et il atteint l’écran.",
          "Most are internal automation or e-mail failures an operator never sees. too_many_attempts is the exception: it is the sign-in throttle 429, and it does reach the screen.",
        ),
      true,
    )}
  </div>
</section>`
    : ""
}
<section>
  <div class="section-head"><h2>${bi("Catalogue complet", "Full catalogue")}</h2></div>
  <div class="capture-grid single">
    ${card(
      bi("DomainError", "DomainError"),
      "server/src/domain/error.rs",
      table([bi("Code", "Code"), bi("Statut", "Status"), "EN", "FR"], rows, [
        "25%",
        "15%",
        "30%",
        "30%",
      ]),
      true,
    )}
  </div>
</section>
<section>
  <div class="section-head"><h2>${bi("Codes levés par le client", "Codes raised by the client")}</h2>
  <p>${bi("Échecs de transport que le serveur ne nomme pas", "Transport failures the server never names")}</p></div>
  <div class="capture-grid single">
    ${card(
      bi("Codes client", "Client codes"),
      "client-web/lib/queries/",
      table(
        [bi("Code", "Code"), "EN", "FR"],
        data.clientCodes.map((row) => [key(row.code, true), muted(row.en), muted(row.fr)]),
        ["34%", "33%", "33%"],
      ) +
        note(
          "Ces codes partagent le namespace errors. L’absence de traduction est ici volontaire : les composants testent tErr.has(code) et retombent sur un libellé générique.",
          "These codes share the errors namespace. A missing translation is deliberate here: components test tErr.has(code) and fall back to a generic label.",
        ),
      true,
    )}
  </div>
</section>`,
  });
}

export function renderApi(routes) {
  const rows = routes.map((route) => [
    cell(tone(METHOD_TONE[route.method] ?? "ghost", route.method)),
    key(route.path, true),
    cell(route.guarded ? tone("ghost", bi("auth", "auth")) : tone("ghost", bi("public", "public"))),
    muted(route.handler),
    muted(route.bodyLimit),
  ]);
  const byMethod = new Map();
  for (const route of routes) byMethod.set(route.method, (byMethod.get(route.method) ?? 0) + 1);

  return page({
    slug: "api",
    titleFr: "Routes HTTP",
    titleEn: "HTTP routes",
    introFr:
      "Chaque route, son verbe, si elle est derrière l’authentification et le plafond de corps qu’elle accepte. Une colonne vide en limite signifie le défaut d’axum, soit 2 Mio.",
    introEn:
      "Every route, its verb, whether it sits behind authentication, and the body ceiling it accepts. An empty limit means the axum default, which is 2 MiB.",
    body: `
${summary([
  [routes.length, "routes", "routes"],
  [routes.filter((route) => route.guarded).length, "authentifiées", "authenticated"],
  [routes.filter((route) => !route.guarded).length, "publiques", "public"],
  [routes.filter((route) => route.bodyLimit).length, "limites explicites", "explicit limits"],
  ...[...byMethod.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 3)
    .map(([method, count]) => [count, method, method]),
])}
<section>
  <div class="section-head"><h2>${bi("Toutes les routes", "Every route")}</h2></div>
  <div class="capture-grid single">
    ${card(
      bi("build_app()", "build_app()"),
      "server/src/lib.rs",
      table(
        [
          bi("Verbe", "Verb"),
          bi("Chemin", "Path"),
          bi("Accès", "Access"),
          bi("Handler", "Handler"),
          bi("Corps", "Body"),
        ],
        rows,
        ["9%", "34%", "10%", "27%", "20%"],
      ) +
        note(
          "Les deux surfaces de conversation acceptent le même plafond de 14 Mio, ce qui laisse passer 10 Mio de pièces jointes une fois encodées en base64 dans du JSON.",
          "Both conversation surfaces accept the same 14 MiB ceiling, which admits 10 MiB of attachments once base64-encoded inside JSON.",
        ),
      true,
    )}
  </div>
</section>`,
  });
}

export function renderEvents(events, ws) {
  return page({
    slug: "events",
    titleFr: "Événements et temps réel",
    titleEn: "Events and realtime",
    introFr:
      "Les événements du domaine, la portée à laquelle chacun est livré, et la trame WebSocket correspondante. Une portée Team atteint tous les membres ; une portée Users ne vise que les deux participants nommés.",
    introEn:
      "Domain events, the scope each is delivered to, and the matching WebSocket frame. A Team scope reaches every member; a Users scope targets only the two named participants.",
    body: `
${summary([
  [events.length, "événements", "events"],
  [events.filter((event) => event.delivery === "Team").length, "portée équipe", "team scope"],
  [
    events.filter((event) => event.delivery === "Users").length,
    "portée participants",
    "participant scope",
  ],
  [ws.commands.length, "commandes client", "client commands"],
  [ws.frames.length, "trames serveur", "server frames"],
  [ws.frames.filter((frame) => !frame.documented).length, "non spécifiées", "unspecified"],
])}
<section>
  <div class="section-head"><h2>${bi("Événements du domaine", "Domain events")}</h2></div>
  <div class="capture-grid single">
    ${card(
      bi("DomainEvent", "DomainEvent"),
      "server/src/domain/event.rs",
      table(
        [
          bi("Variante", "Variant"),
          bi("Portée", "Scope"),
          bi("Trame", "Frame"),
          bi("Intention", "Intent"),
        ],
        events.map((event) => [
          key(event.variant, true),
          cell(
            event.delivery === "Team"
              ? tone("info", bi("équipe", "team"))
              : tone("warning", bi("participants", "participants")),
          ),
          key(event.frame ?? "—"),
          muted(event.doc),
        ]),
        ["24%", "14%", "24%", "38%"],
      ),
      true,
    )}
  </div>
</section>
<section>
  <div class="section-head"><h2>${bi("Protocole WebSocket", "WebSocket protocol")}</h2>
  <p>${bi("Documenté signifie : nommé dans WEBSOCKET_SPEC.md", "Documented means: named in WEBSOCKET_SPEC.md")}</p></div>
  <div class="capture-grid">
    ${card(
      bi("Commandes client", "Client commands"),
      "server/src/handlers/ws.rs",
      table(
        [bi("Type", "Type"), bi("Spécifiée", "Specified")],
        ws.commands.map((command) => [key(command.type, true), cell(yesNo(command.documented))]),
        ["72%", "28%"],
      ),
    )}
    ${card(
      bi("Trames serveur", "Server frames"),
      "server/src/adapters/ws/protocol.rs",
      table(
        [bi("Type", "Type"), bi("Spécifiée", "Specified")],
        ws.frames.map((frame) => [key(frame.frame, true), cell(yesNo(frame.documented))]),
        ["72%", "28%"],
      ),
    )}
  </div>
</section>`,
  });
}
