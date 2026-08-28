// --- tooling/inventory/extract-web.mjs ---
//
// Interface families. These have no compiler guarantee behind them, so every
// extractor states what it counted and the pages show the raw number rather
// than a verdict.

import fs from "node:fs";
import path from "node:path";

import { expectAtLeast, read, readJson, ROOT } from "./sources.mjs";

function walk(relative, predicate) {
  const files = [];
  const visit = (current) => {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const target = path.join(current, entry.name);
      if (entry.isDirectory()) visit(target);
      else if (predicate(entry.name)) files.push(target);
    }
  };
  visit(path.join(ROOT, relative));
  return files;
}

const isComponent = (name) => /\.tsx?$/.test(name) && !name.includes(".test.");

/** Design tokens declared on `:root`, grouped by their family prefix. */
export function tokens() {
  const css = read("client-web/app/globals.css");
  const rootBlock = /:root\s*\{([\s\S]*?)\n\}/.exec(css);
  const declarations = [...(rootBlock?.[1] ?? "").matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);/g)].map(
    (match) => ({
      name: match[1],
      value: match[2].trim(),
    }),
  );
  expectAtLeast("design tokens", declarations.length, 30);

  const families = new Map();
  for (const token of declarations) {
    const family = token.name.split("-")[0];
    if (!families.has(family)) families.set(family, []);
    families.get(family).push(token);
  }
  return {
    total: declarations.length,
    families: [...families.entries()]
      .map(([family, items]) => ({ family, items }))
      .sort((a, b) => b.items.length - a.items.length || a.family.localeCompare(b.family)),
  };
}

/** Every `components/ui` primitive with the variant and size unions it accepts. */
export function primitives() {
  const dir = path.join(ROOT, "client-web/components/ui");
  const files = fs.readdirSync(dir).filter(isComponent).sort();
  expectAtLeast("ui primitives", files.length, 10);

  return files.map((file) => {
    const body = fs.readFileSync(path.join(dir, file), "utf8");
    const unions = [
      ...body.matchAll(/export type (\w+)\s*=\s*((?:"[^"]+"\s*\|\s*)+"[^"]+")\s*;/g),
    ].map((match) => ({
      name: match[1],
      values: [...match[2].matchAll(/"([^"]+)"/g)].map((value) => value[1]),
    }));
    // `const variantClasses: Record<ButtonVariant, string> = { primary: ... }`
    const maps = [
      ...body.matchAll(
        /const (\w+Classes)\s*:\s*Record<\s*([\w"|\s]+),\s*string\s*>\s*=\s*\{([\s\S]*?)\n\}/g,
      ),
    ].map((match) => ({
      name: match[1],
      values: [...match[3].matchAll(/^\s{2}([a-z][\w]*)\s*:/gm)].map((value) => value[1]),
    }));
    const inlineProps = [
      ...body.matchAll(/\b(variant|size|tone)\?\?\s*:\s*((?:"[^"]+"\s*\|\s*)+"[^"]+")/g),
    ].map((match) => ({
      name: match[1],
      values: [...match[2].matchAll(/"([^"]+)"/g)].map((value) => value[1]),
    }));

    const axes = new Map();
    for (const group of [...unions, ...maps, ...inlineProps]) {
      const axis = /variant/i.test(group.name)
        ? "variant"
        : /size/i.test(group.name)
          ? "size"
          : /tone/i.test(group.name)
            ? "tone"
            : group.name;
      const current = axes.get(axis) ?? new Set();
      for (const value of group.values) current.add(value);
      axes.set(axis, current);
    }

    return {
      component: file.replace(/\.tsx?$/, ""),
      file: `client-web/components/ui/${file}`,
      hasTest: fs.existsSync(path.join(dir, file.replace(/\.tsx?$/, ".test.tsx"))),
      axes: [...axes.entries()].map(([axis, values]) => ({ axis, values: [...values] })),
      lines: body.split("\n").length,
    };
  });
}

/**
 * `data-*` attributes are the contract the browser suite selects on. A declared
 * attribute no spec reads is either dead weight or an untested guarantee.
 */
export function domContract() {
  const declared = new Map();
  for (const file of walk("client-web", isComponent)) {
    const body = fs.readFileSync(file, "utf8");
    for (const match of body.matchAll(/"?(data-[a-z][a-z0-9-]*)"?\s*[:=]/g)) {
      const relative = path.relative(ROOT, file);
      if (!declared.has(match[1])) declared.set(match[1], new Set());
      declared.get(match[1]).add(relative);
    }
  }

  const used = new Set();
  for (const file of walk("tooling/e2e", (name) => name.endsWith(".spec.ts"))) {
    const body = fs.readFileSync(file, "utf8");
    for (const match of body.matchAll(/\[(data-[a-z][a-z0-9-]*)[\]=^*$~]/g)) used.add(match[1]);
  }

  expectAtLeast("declared data attributes", declared.size, 20);
  return [...declared.entries()]
    .map(([attribute, files]) => ({
      attribute,
      files: [...files].sort(),
      covered: used.has(attribute),
    }))
    .sort(
      (a, b) => Number(a.covered) - Number(b.covered) || a.attribute.localeCompare(b.attribute),
    );
}

/** Namespaces, key counts and the word budget the ratchet enforces. */
export function i18n() {
  const locales = ["en", "fr"];
  const messages = Object.fromEntries(
    locales.map((locale) => [locale, readJson(`client-web/messages/${locale}.json`)]),
  );
  const budget = read("client-web/i18n/text-budget.test.ts");

  const words = (value) => {
    if (typeof value === "string")
      return (value.replace(/\{[^}]*\}/g, " ").match(/[A-Za-z0-9'’]+/g) ?? []).length;
    if (value && typeof value === "object")
      return Object.values(value).reduce((total, entry) => total + words(entry), 0);
    return 0;
  };
  const keys = (value) => {
    if (typeof value !== "object" || value === null) return 1;
    return Object.values(value).reduce((total, entry) => total + keys(entry), 0);
  };

  const ceilings = {};
  for (const locale of locales) {
    const block = new RegExp(`${locale}:\\s*\\{([\\s\\S]*?)\\n  \\}`).exec(budget);
    ceilings[locale] = Object.fromEntries(
      [...(block?.[1] ?? "").matchAll(/(\w+)\s*:\s*(\d+)/g)].map((match) => [
        match[1],
        Number(match[2]),
      ]),
    );
  }

  const namespaces = Object.keys(messages.en).map((namespace) => ({
    namespace,
    keys: keys(messages.en[namespace]),
    missingInFr: keys(messages.en[namespace]) - keys(messages.fr[namespace] ?? {}),
    locales: Object.fromEntries(
      locales.map((locale) => [
        locale,
        {
          words: words(messages[locale][namespace]),
          ceiling: ceilings[locale][namespace] ?? null,
          slack: (ceilings[locale][namespace] ?? 0) - words(messages[locale][namespace]),
        },
      ]),
    ),
  }));

  expectAtLeast("i18n namespaces", namespaces.length, 10);
  return {
    namespaces: namespaces.sort((a, b) => b.locales.en.words - a.locales.en.words),
    totalKeys: namespaces.reduce((total, entry) => total + entry.keys, 0),
  };
}

/** The automation catalog exactly as the running server publishes it. */
export async function automations(baseUrl) {
  try {
    const response = await fetch(`${baseUrl}/about.json`, { signal: AbortSignal.timeout(2500) });
    if (!response.ok) return { available: false, reason: `HTTP ${response.status}`, services: [] };
    const body = await response.json();
    const services = body?.server?.services ?? [];
    return {
      available: services.length > 0,
      reason: services.length > 0 ? null : "the running server published an empty catalog",
      services: services.map((service) => ({
        name: service.name,
        label: service.label,
        actions: service.actions ?? [],
        reactions: service.reactions ?? [],
      })),
    };
  } catch (error) {
    return { available: false, reason: `no server on ${baseUrl} (${error.message})`, services: [] };
  }
}
