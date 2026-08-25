// --- tooling/inventory/sources.mjs ---
//
// Reading helpers shared by every extractor. Each family below is derived from
// a source the compiler or a test already guarantees: an exhaustive `match`, a
// contract file with a conformance test, or the live `/about.json`. Nothing is
// transcribed by hand, so an inventory page cannot quietly disagree with the
// code it documents.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

export function read(relative) {
  return fs.readFileSync(path.join(ROOT, relative), "utf8");
}

export function readJson(relative) {
  return JSON.parse(read(relative));
}

export function exists(relative) {
  return fs.existsSync(path.join(ROOT, relative));
}

/** Collapse Rust formatting so one match arm is one line, whatever rustfmt did. */
export function flatten(source) {
  return source.replace(/\s+/g, " ");
}

/**
 * Body of a `fn name(...)` up to its closing brace, by brace counting. Used to
 * scope a regex to one exhaustive match instead of the whole file.
 */
export function fnBody(source, signature) {
  const start = source.indexOf(signature);
  if (start === -1) throw new Error(`inventory: signature not found: ${signature}`);
  const open = source.indexOf("{", start);
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(open + 1, index);
    }
  }
  throw new Error(`inventory: unbalanced braces after: ${signature}`);
}

/** Body of an `enum Name {` / `struct Name {` declaration. */
export function blockBody(source, declaration) {
  return fnBody(source, declaration);
}

/** `PascalCase` -> `snake_case`, matching serde's `rename_all`. */
export function snake(identifier) {
  return identifier
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1_$2")
    .toLowerCase();
}

/** `can_do_thing` -> `canDoThing`, matching serde's camelCase contract. */
export function camel(identifier) {
  return identifier.replace(/_([a-z0-9])/g, (_, character) => character.toUpperCase());
}

/** Leading `///` doc comment for a declaration line inside an enum body. */
export function docFor(body, variant) {
  const lines = body.split("\n");
  const index = lines.findIndex((line) => new RegExp(`^\\s*${variant}\\b`).test(line));
  if (index === -1) return null;
  const doc = [];
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const line = lines[cursor].trim();
    if (!line.startsWith("///")) break;
    doc.unshift(line.replace(/^\/\/\/\s?/, ""));
  }
  return doc.length > 0 ? doc.join(" ") : null;
}

/** Fail loudly when a parser silently stops matching after a refactor. */
export function expectAtLeast(label, actual, minimum) {
  if (actual < minimum) {
    throw new Error(
      `inventory: ${label} yielded ${actual} entries, expected at least ${minimum}. ` +
        `The source shape changed — fix the extractor rather than lowering the floor.`,
    );
  }
  return actual;
}
