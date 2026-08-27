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

function hasMessage(messages: Messages, key: string): boolean {
  let current: unknown = messages;
  for (const segment of key.split(".")) {
    if (typeof current !== "object" || current === null || !(segment in current)) return false;
    current = (current as Messages)[segment];
  }
  return typeof current === "string";
}

function translationKeyViolations(file: string, messages: Messages): string[] {
  const source = fs.readFileSync(file, "utf8");
  const tree = ts.createSourceFile(
    file,
    source,
    ts.ScriptTarget.Latest,
    true,
    file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const translators = new Map<string, string>();
  const collections = new Map<string, string[]>();
  const violations: string[] = [];

  const stringValues = (expression: ts.Expression): string[] | undefined => {
    if (ts.isStringLiteralLike(expression)) return [expression.text];
    if (ts.isArrayLiteralExpression(expression)) {
      const values = expression.elements.flatMap((element) =>
        ts.isExpression(element) ? (stringValues(element) ?? []) : [],
      );
      return values.length === expression.elements.length ? values : undefined;
    }
    if (ts.isObjectLiteralExpression(expression)) {
      const values = expression.properties.flatMap((property) =>
        ts.isPropertyAssignment(property) ? (stringValues(property.initializer) ?? []) : [],
      );
      return values.length === expression.properties.length ? values : undefined;
    }
    if (ts.isAsExpression(expression) || ts.isSatisfiesExpression(expression)) {
      return stringValues(expression.expression);
    }
    return undefined;
  };

  const collect = (node: ts.Node) => {
    if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
      const initializer = ts.isAwaitExpression(node.initializer)
        ? node.initializer.expression
        : node.initializer;
      if (ts.isCallExpression(initializer)) {
        if (
          initializer.expression.getText(tree) === "useTranslations" &&
          initializer.arguments.length === 1 &&
          ts.isStringLiteralLike(initializer.arguments[0])
        ) {
          translators.set(node.name.text, initializer.arguments[0].text);
        }
        if (
          initializer.expression.getText(tree) === "getTranslations" &&
          initializer.arguments.length === 1 &&
          ts.isObjectLiteralExpression(initializer.arguments[0])
        ) {
          const namespace = initializer.arguments[0].properties.find(
            (property): property is ts.PropertyAssignment =>
              ts.isPropertyAssignment(property) && property.name.getText(tree) === "namespace",
          )?.initializer;
          if (namespace && ts.isStringLiteralLike(namespace)) {
            translators.set(node.name.text, namespace.text);
          }
        }
      }

      const values = stringValues(initializer);
      if (values) collections.set(node.name.text, values);
    }
    ts.forEachChild(node, collect);
  };
  collect(tree);

  const resolve = (
    expression: ts.Expression,
    dynamic: Map<string, string[]>,
  ): string[] | undefined => {
    if (ts.isStringLiteralLike(expression)) return [expression.text];
    if (ts.isIdentifier(expression))
      return dynamic.get(expression.text) ?? collections.get(expression.text);
    if (ts.isElementAccessExpression(expression) && ts.isIdentifier(expression.expression)) {
      return collections.get(expression.expression.text);
    }
    if (ts.isConditionalExpression(expression)) {
      const left = resolve(expression.whenTrue, dynamic);
      const right = resolve(expression.whenFalse, dynamic);
      return left && right ? [...left, ...right] : undefined;
    }
    if (ts.isAsExpression(expression) || ts.isSatisfiesExpression(expression)) {
      return resolve(expression.expression, dynamic);
    }
    return undefined;
  };

  const report = (node: ts.Node, detail: string) => {
    const line = tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
    violations.push(`${path.relative(process.cwd(), file)}:${line}: ${detail}`);
  };

  const visit = (node: ts.Node, dynamic = new Map<string, string[]>()) => {
    if (
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression) &&
      node.expression.name.text === "map" &&
      node.arguments.length > 0 &&
      (ts.isArrowFunction(node.arguments[0]) || ts.isFunctionExpression(node.arguments[0]))
    ) {
      const values = ts.isIdentifier(node.expression.expression)
        ? collections.get(node.expression.expression.text)
        : stringValues(node.expression.expression);
      const callback = node.arguments[0];
      const parameter = callback.parameters[0]?.name;
      if (values && parameter && ts.isIdentifier(parameter)) {
        const nested = new Map(dynamic);
        nested.set(parameter.text, values);
        visit(callback.body, nested);
        return;
      }
    }

    if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      translators.has(node.expression.text) &&
      node.arguments.length > 0
    ) {
      const namespace = translators.get(node.expression.text)!;
      const keys = resolve(node.arguments[0], dynamic);
      if (keys) {
        for (const key of new Set(keys)) {
          const fullKey = `${namespace}.${key}`;
          if (!hasMessage(messages, fullKey)) report(node, `missing message ${fullKey}`);
        }
      }
    }

    ts.forEachChild(node, (child) => visit(child, dynamic));
  };
  visit(tree);
  return violations;
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
  "Common.gifAlt",
  "Auth.logoWordmarkAlt",
  "Automations.action",
  "Automations.durationMs",
  "Automations.reaction",
  "DirectMessages.message",
  "DirectMessages.roomTitle",
  "Incidents.activityEventCount",
  "Incidents.fieldDescription",
  "Incidents.colIncident",
  "Incidents.incidentBreadcrumb",
  "Incidents.linkedReleases",
  "Incidents.moreActions",
  "Incidents.title",
  // Pure interpolation: the body of a direct-message notification is the
  // message itself, so there is no prose to translate.
  "Notifications.directMessageBody",
  // Same word in both languages.
  "Settings.notifications",
  // Two numbers and a slash: there is nothing to translate.
  "Sidebar.tourProgress",
  "Onboarding.operatorNamePlaceholder",
  "Onboarding.teamNamePlaceholder",
  "Onboarding.invitationCodePlaceholder",
  "Teams.namePlaceholder",
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
  "Teams.calendar.incident",
  "Teams.calendar.release",
  "Teams.overviewViews.incidents",
  "Teams.overviewViews.releases",
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

  it("resolves every statically referenced translation key", () => {
    const roots = ["app", "components", "lib"].map((directory) =>
      path.join(process.cwd(), directory),
    );
    const violations = roots
      .flatMap(sourceFiles)
      .flatMap((file) => translationKeyViolations(file, en));

    expect(violations, violations.join("\n")).toEqual([]);
  });
});
