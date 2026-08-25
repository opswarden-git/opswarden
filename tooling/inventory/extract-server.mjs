// --- tooling/inventory/extract-server.mjs ---
//
// Server families. Every list here comes from an exhaustive Rust `match`, so
// the compiler already guarantees completeness: the parser only has to read a
// list it cannot be missing entries from.

import fs from "node:fs";
import path from "node:path";

import {
  blockBody,
  docFor,
  expectAtLeast,
  flatten,
  fnBody,
  read,
  readJson,
  ROOT,
  snake,
} from "./sources.mjs";

/** 3 roles x 17 capabilities, from the contract `capabilities.rs` is tested against. */
export function capabilities() {
  const contract = readJson("contracts/role-capabilities.json");
  const struct = blockBody(
    read("server/src/domain/capabilities.rs"),
    "pub struct TeamCapabilities",
  );
  const fields = [...struct.matchAll(/pub (can_[a-z_]+): bool/g)].map((match) => match[1]);
  expectAtLeast("capability fields", fields.length, 15);

  const roles = Object.keys(contract);
  return {
    roles,
    fields: fields.map((field) => ({
      field,
      label: field.replace(/^can_/, "").replace(/_/g, " "),
      byRole: Object.fromEntries(
        roles.map((role) => [
          role,
          contract[role][field.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase())],
        ]),
      ),
    })),
  };
}

/** 72 domain errors x code, HTTP status, and both locales. */
export function errors() {
  const domain = read("server/src/domain/error.rs");
  const enumBody = blockBody(domain, "pub enum DomainError");
  const codeArms = flatten(fnBody(domain, "pub fn code(&self)"));
  const statusArms = flatten(
    fnBody(read("server/src/handlers/error.rs"), "fn into_response(self)"),
  );
  const en = readJson("client-web/messages/en.json").errors ?? {};
  const fr = readJson("client-web/messages/fr.json").errors ?? {};

  const codes = new Map();
  for (const match of codeArms.matchAll(
    /DomainError::(\w+)\s*(?:\{[^}]*\})?\s*=>\s*"([a-z0-9_]+)"/g,
  )) {
    codes.set(match[1], match[2]);
  }
  expectAtLeast("domain error codes", codes.size, 60);

  // Status arms group variants with `|`, may destructure, and rustfmt wraps the
  // long ones in a block: `=> { (StatusCode::X, "...") }` as well as `=> (...)`.
  const statuses = new Map();
  for (const arm of statusArms.matchAll(
    /((?:DomainError::\w+\s*(?:\{[^}]*\})?\s*\|\s*)*DomainError::\w+\s*(?:\{[^}]*\})?)\s*=>\s*\{?\s*\(\s*StatusCode::(\w+)/g,
  )) {
    for (const variant of arm[1].matchAll(/DomainError::(\w+)/g)) statuses.set(variant[1], arm[2]);
  }

  const rows = [...codes.entries()].map(([variant, code]) => ({
    variant,
    code,
    status: statuses.get(variant) ?? "INTERNAL_SERVER_ERROR",
    statusInferred: !statuses.has(variant),
    doc: docFor(enumBody, variant),
    en: en[code] ?? null,
    fr: fr[code] ?? null,
  }));

  // The interface also raises its own codes for transport failures the server
  // never names. They share the `errors` namespace, so a key is only orphaned
  // when neither side can produce it.
  const clientCodes = clientErrorCodes();
  const serverCodes = new Set(codes.values());

  return {
    rows,
    clientCodes: [...clientCodes].sort().map((code) => ({
      code,
      en: en[code] ?? null,
      fr: fr[code] ?? null,
    })),
    untranslated: rows.filter((row) => !row.en || !row.fr).map((row) => row.code),
    orphanKeys: Object.keys(en).filter((key) => !serverCodes.has(key) && !clientCodes.has(key)),
  };
}

/** Codes the web client throws itself, e.g. `throw new Error("create_team_failed")`. */
function clientErrorCodes() {
  const dir = path.join(ROOT, "client-web/lib");
  const codes = new Set();
  const walk = (current) => {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const target = path.join(current, entry.name);
      if (entry.isDirectory()) walk(target);
      else if (/\.tsx?$/.test(entry.name) && !entry.name.includes(".test.")) {
        const body = fs.readFileSync(target, "utf8");
        // `?? "incidents"` is a query key, not an error code: keep the shape
        // every real code has, a snake_case identifier of two or more words.
        for (const match of body.matchAll(
          /(?:new Error\(|\?\?\s*)"([a-z][a-z0-9]*(?:_[a-z0-9]+)+)"/g,
        )) {
          codes.add(match[1]);
        }
      }
    }
  };
  walk(dir);
  return codes;
}

/** 17 domain events x delivery scope x wire frame. */
export function events() {
  const source = read("server/src/domain/event.rs");
  const enumBody = blockBody(source, "pub enum DomainEvent");
  const deliveryArms = flatten(fnBody(source, "pub fn delivery(&self)"));
  const wire = flatten(fnBody(read("server/src/adapters/ws/protocol.rs"), "pub fn to_wire"));

  const variants = [...enumBody.matchAll(/^\s{4}(\w+)\s*[{,]/gm)].map((match) => match[1]);
  expectAtLeast("domain events", variants.length, 15);

  const delivery = new Map();
  for (const arm of deliveryArms.matchAll(
    /((?:DomainEvent::\w+\s*(?:\{[^}]*\})?\s*\|\s*)*DomainEvent::\w+\s*(?:\{[^}]*\})?)\s*=>\s*EventDelivery::(\w+)/g,
  )) {
    for (const variant of arm[1].matchAll(/DomainEvent::(\w+)/g)) delivery.set(variant[1], arm[2]);
  }

  const frames = new Map();
  for (const arm of wire.matchAll(
    /DomainEvent::(\w+)\s*(?:\{[^}]*\})?\s*=>\s*(?:return\s+)?(?:(\w+_wire)|json!\(\s*\{\s*"type"\s*:\s*"([a-z_]+)")/g,
  )) {
    frames.set(arm[1], arm[3] ?? `${arm[2]}()`);
  }

  return variants.map((variant) => ({
    variant,
    doc: docFor(enumBody, variant),
    delivery: delivery.get(variant) ?? "unknown",
    frame: frames.get(variant) ?? null,
  }));
}

/** Client commands and every server frame the protocol can emit. */
export function websocket() {
  const handler = read("server/src/handlers/ws.rs");
  const protocol = read("server/src/adapters/ws/protocol.rs");

  const commands = [
    ...blockBody(handler, "enum ClientCommand").matchAll(/^\s{4}(\w+)\s*[{,]/gm),
  ].map((match) => ({ variant: match[1], type: snake(match[1]) }));
  expectAtLeast("ws client commands", commands.length, 6);

  const frames = [
    ...new Set([...protocol.matchAll(/"type"\s*:\s*"([a-z_]+)"/g)].map((m) => m[1])),
  ].sort();
  expectAtLeast("ws server frames", frames.length, 20);

  // A frame the spec never names is a contract the client cannot rely on.
  const spec = read("WEBSOCKET_SPEC.md");
  return {
    commands: commands.map((command) => ({ ...command, documented: spec.includes(command.type) })),
    frames: frames.map((frame) => ({ frame, documented: spec.includes(frame) })),
  };
}

/** Every route, its verb, whether it sits behind auth, and its body ceiling. */
export function routes() {
  const source = read("server/src/lib.rs");
  const body = fnBody(source, "pub fn build_app");
  const protectedStart = body.indexOf("let protected_routes");
  const publicStart = body.indexOf("Router::new()", body.indexOf("}", protectedStart));

  // A route's chain runs until the next `.route(`/`.merge(`/`.with_state(`.
  // `.layer(` must NOT end it: that is exactly where a body ceiling is declared.
  const parse = (segment, guarded) =>
    [
      ...segment.matchAll(
        /\.route\(\s*"([^"]+)"\s*,([\s\S]*?)(?=\.route\(\s*"|\.merge\(|\.with_state\(|$)/g,
      ),
    ].flatMap((match) => {
      const [, routePath, chain] = match;
      const limit = /DefaultBodyLimit::max\(\s*([^)]+?)\s*\)/.exec(chain);
      return [...chain.matchAll(/\b(get|post|put|patch|delete)\(\s*handlers::([\w:]+)/g)].map(
        (verb) => ({
          path: routePath,
          method: verb[1].toUpperCase(),
          handler: verb[2],
          guarded,
          bodyLimit: limit ? limit[1].replace(/\s+/g, " ").replace(/,$/, "") : null,
        }),
      );
    });

  const guarded = parse(body.slice(protectedStart, publicStart), true);
  const open = parse(body.slice(publicStart), false);
  const all = [...guarded, ...open];
  expectAtLeast("routes", all.length, 50);
  return all.sort((a, b) => a.path.localeCompare(b.path) || a.method.localeCompare(b.method));
}

/** Incident and release lifecycles, read from the transition guards themselves. */
export function stateMachines() {
  const incident = read("server/src/domain/incident.rs");
  const statuses = [
    ...blockBody(incident, "pub enum IncidentStatus").matchAll(/^\s{4}(\w+),/gm),
  ].map((match) => snake(match[1]));
  const severities = [...blockBody(incident, "pub enum Severity").matchAll(/^\s{4}(\w+),/gm)].map(
    (match) => snake(match[1]),
  );
  const release = read("server/src/domain/release.rs");
  const releaseStates = [
    ...blockBody(release, "pub enum ReleaseState").matchAll(/^\s{4}(\w+),/gm),
  ].map((match) => snake(match[1]));

  expectAtLeast("incident statuses", statuses.length, 4);
  return { incidentStatuses: statuses, severities, releaseStates };
}

/** Bounds a reviewer would otherwise have to hunt across six files. */
export function limits() {
  const conversation = read("server/src/domain/conversation.rs");
  const activity = read("server/src/app/incident/list_activity.rs");
  const conversationList = read("server/src/app/private_message/list_private_messages.rs");
  const timeline = read("server/src/domain/timeline.rs");

  // Several bounds are declared as an alias of a shared constant, e.g.
  // `MAX_TIMELINE_ENTRY_LEN: usize = MAX_MESSAGE_LEN`. Resolve one hop so the
  // board shows the value that actually applies, not the indirection.
  const constant = (source, name, depth = 0) => {
    const match = new RegExp(`${name}\\s*:\\s*\\w+\\s*=\\s*([^;]+);`).exec(source);
    if (!match) return null;
    const value = match[1].trim();
    if (depth < 2 && /^[A-Z][A-Z0-9_]*$/.test(value)) {
      return (
        constant(conversation, value, depth + 1) ?? constant(source, value, depth + 1) ?? value
      );
    }
    return value;
  };

  const mediaTypes = [
    ...fnBody(conversation, "fn allowed_media_type").matchAll(/"([a-z0-9.\-+/]+)"/g),
  ].map((match) => match[1]);

  // Reuse the parsed router rather than re-scanning lib.rs: a loose regex here
  // matched any quoted string near a layer and produced nonsense rows.
  const bodyLimits = [
    ...new Map(
      routes()
        .filter((route) => route.bodyLimit)
        .map((route) => [route.path, { route: route.path, limit: route.bodyLimit }]),
    ).values(),
  ];

  return {
    values: [
      ["Message body", constant(conversation, "MAX_MESSAGE_LEN"), "domain/conversation.rs"],
      [
        "Attachments per message",
        constant(conversation, "MAX_MESSAGE_ATTACHMENTS"),
        "domain/conversation.rs",
      ],
      [
        "Bytes per attachment",
        constant(conversation, "MAX_MESSAGE_ATTACHMENT_BYTES"),
        "domain/conversation.rs",
      ],
      [
        "Bytes per message, combined",
        constant(conversation, "MAX_MESSAGE_ATTACHMENTS_TOTAL_BYTES"),
        "domain/conversation.rs",
      ],
      ["Timeline entry body", constant(timeline, "MAX_TIMELINE_ENTRY_LEN"), "domain/timeline.rs"],
      [
        "Activity page, default",
        constant(activity, "DEFAULT_ACTIVITY_LIMIT"),
        "app/incident/list_activity.rs",
      ],
      [
        "Activity page, maximum",
        constant(activity, "MAX_ACTIVITY_LIMIT"),
        "app/incident/list_activity.rs",
      ],
      [
        "Conversation page, default",
        constant(conversationList, "DEFAULT_CONVERSATION_LIMIT"),
        "app/private_message/list_private_messages.rs",
      ],
      [
        "Conversation page, maximum",
        constant(conversationList, "MAX_CONVERSATION_LIMIT"),
        "app/private_message/list_private_messages.rs",
      ],
    ].filter(([, value]) => value !== null),
    mediaTypes,
    bodyLimits,
    reactions: [...timeline.matchAll(/AVAILABLE_REACTIONS[^=]+=\s*\[([^\]]+)\]/g)].flatMap(
      (match) => [...match[1].matchAll(/"([^"]+)"/g)].map((emoji) => emoji[1]),
    ),
  };
}

/** Conversation capability grid: the parity claim, stated by the domain itself. */
export function conversationFeatures() {
  const source = read("server/src/domain/conversation.rs");
  const all = [...blockBody(source, "pub enum ConversationFeature").matchAll(/^\s{4}(\w+),/gm)].map(
    (match) => snake(match[1]),
  );
  const group = (name) =>
    [
      ...(new RegExp(`const ${name}[^=]+=\\s*\\[([^\\]]+)\\]`).exec(source)?.[1] ?? "").matchAll(
        /ConversationFeature::(\w+)/g,
      ),
    ].map((match) => snake(match[1]));

  return { all, direct: group("DIRECT_FEATURES"), incident: group("INCIDENT_FEATURES") };
}

/** Forward-only migration ledger with the phase each file declares. */
export function migrations() {
  const dir = path.join(ROOT, "server/migrations");
  const files = fs
    .readdirSync(dir)
    .filter((name) => name.endsWith(".sql"))
    .sort();
  expectAtLeast("migrations", files.length, 20);

  return files.map((file) => {
    const body = fs.readFileSync(path.join(dir, file), "utf8");
    const phase = /migration-phase=(\w+)/.exec(body);
    const summary = body
      .split("\n")
      .filter((line) => line.startsWith("--") && !line.includes("migration-phase"))
      .map((line) => line.replace(/^--\s?/, "").trim())
      .filter(Boolean)
      .join(" ");
    return {
      file,
      number: file.slice(0, 4),
      phase: phase ? phase[1] : null,
      summary: summary || null,
      statements: (body.match(/^\s*(create|alter|drop|insert|update)\b/gim) ?? []).length,
    };
  });
}
