import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";
import en from "../messages/en.json";
import fr from "../messages/fr.json";

type Messages = Record<string, unknown>;

function leafEntries(value: Messages, prefix = ""): Array<[string, string]> {
  return Object.entries(value).flatMap(([key, child]) => {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    return typeof child === "string" ? [[fullKey, child]] : leafEntries(child as Messages, fullKey);
  });
}

function argumentsOf(message: string): string[] {
  const result: string[] = [];
  let depth = 0;
  for (let index = 0; index < message.length; index += 1) {
    if (message[index] === "{") {
      if (depth === 0) {
        const argument = message.slice(index + 1).match(/^([a-zA-Z][a-zA-Z0-9_]*)/)?.[1];
        if (argument) result.push(argument);
      }
      depth += 1;
    } else if (message[index] === "}") {
      depth = Math.max(0, depth - 1);
    }
  }
  return result.sort();
}

function sourceFiles(directory: string): string[] {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) return sourceFiles(target);
    return /\.(ts|tsx)$/.test(entry.name) && !/\.test\.(ts|tsx)$/.test(entry.name) ? [target] : [];
  });
}

const visibleAttributes = new Set([
  "alt",
  "aria-label",
  "copiedLabel",
  "description",
  "errorLabel",
  "label",
  "message",
  "placeholder",
  "title",
]);
const visibleProperties = new Set([
  "desc",
  "description",
  "label",
  "message",
  "placeholder",
  "title",
]);
const stableErrorCode = /^[a-z0-9_]+$/;
const hasLetters = (value: string) => /[A-Za-zÀ-ÿ]/.test(value);
const sameTextAllowed = new Set([
  "Auth.email",
  "Auth.logoWordmarkAlt",
  "Automations.action",
  "Automations.colAction",
  "Automations.colReaction",
  "Automations.durationMs",
  "Automations.reaction",
  "DirectMessages.message",
  "Incidents.fieldDescription",
  "Incidents.gifAlt",
  "Incidents.incidentBreadcrumb",
  "Incidents.title",
  "Onboarding.operatorNamePlaceholder",
  "Onboarding.organizationPlaceholder",
  "Onboarding.timezoneParis",
  "Releases.colAction",
  "Releases.colRelease",
  "Releases.progressCount",
  "Releases.title",
  "Settings.emailLabel",
  "Settings.englishShort",
  "Settings.frenchShort",
  "Sidebar.incidents",
  "Sidebar.logoWordmarkAlt",
  "Sidebar.releases",
  "Teams.banPermanent",
  "Teams.code",
  "Teams.invitation",
  "Teams.invitationCodePlaceholder",
  "Teams.roleManager",
]);

function hardcodedSurfaceStrings(file: string): string[] {
  const source = fs.readFileSync(file, "utf8");
  const tree = ts.createSourceFile(
    file,
    source,
    ts.ScriptTarget.Latest,
    true,
    file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const violations: string[] = [];
  const report = (node: ts.Node, value: string) => {
    const line = tree.getLineAndCharacterOfPosition(node.getStart()).line + 1;
    violations.push(`${path.relative(process.cwd(), file)}:${line}: ${JSON.stringify(value)}`);
  };

  const visit = (node: ts.Node) => {
    if (ts.isJsxText(node)) {
      const value = node.getText().replace(/\s+/g, " ").trim();
      if (hasLetters(value)) report(node, value);
    }

    if (ts.isJsxAttribute(node) && visibleAttributes.has(node.name.getText())) {
      const initializer = node.initializer;
      if (initializer && ts.isStringLiteral(initializer) && hasLetters(initializer.text)) {
        report(node, initializer.text);
      }
    }

    if (
      ts.isPropertyAssignment(node) &&
      visibleProperties.has(node.name.getText()) &&
      ts.isStringLiteralLike(node.initializer) &&
      hasLetters(node.initializer.text)
    ) {
      report(node, node.initializer.text);
    }

    if (
      ts.isNewExpression(node) &&
      node.expression.getText() === "Error" &&
      node.arguments?.length === 1 &&
      ts.isStringLiteral(node.arguments[0]) &&
      !stableErrorCode.test(node.arguments[0].text)
    ) {
      report(node, node.arguments[0].text);
    }

    if (
      ts.isCallExpression(node) &&
      node.expression.getText() === "notifyDesktop" &&
      node.arguments.some(
        (argument) =>
          (ts.isStringLiteralLike(argument) || ts.isNoSubstitutionTemplateLiteral(argument)) &&
          hasLetters(argument.text),
      )
    ) {
      report(node, "hard-coded desktop notification");
    }

    ts.forEachChild(node, visit);
  };
  visit(tree);
  return violations;
}

describe("English/French translation completeness", () => {
  it("keeps identical, non-empty leaf keys and interpolation arguments", () => {
    const english = new Map(leafEntries(en));
    const french = new Map(leafEntries(fr));

    expect([...french.keys()].sort()).toEqual([...english.keys()].sort());
    for (const [key, englishMessage] of english) {
      const frenchMessage = french.get(key);
      expect(englishMessage.trim(), `${key} is empty in English`).not.toBe("");
      expect(frenchMessage?.trim(), `${key} is empty in French`).not.toBe("");
      expect(argumentsOf(frenchMessage ?? ""), `${key} uses different arguments`).toEqual(
        argumentsOf(englishMessage),
      );
      if (frenchMessage === englishMessage) {
        expect(sameTextAllowed.has(key), `${key} is unintentionally identical`).toBe(true);
      }
    }
  });

  it("rejects hard-coded user-facing strings in application sources", () => {
    const roots = ["app", "components", "lib"].map((directory) =>
      path.join(process.cwd(), directory),
    );
    const violations = roots.flatMap(sourceFiles).flatMap(hardcodedSurfaceStrings);

    expect(violations, violations.join("\n")).toEqual([]);
  });
});
