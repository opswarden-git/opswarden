import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const nativeControls = new Set(["input", "select", "textarea"]);
const nativeInteractiveElements = new Set([
  "a",
  "button",
  "details",
  "input",
  "option",
  "select",
  "summary",
  "textarea",
]);

function sourceFiles(directory: string): string[] {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) return sourceFiles(target);
    return entry.name.endsWith(".tsx") && !entry.name.endsWith(".test.tsx") ? [target] : [];
  });
}

const productFiles = ["app", "components"].flatMap((directory) =>
  sourceFiles(path.join(process.cwd(), directory)),
);

function jsxAttributes(node: ts.JsxOpeningLikeElement): ts.JsxAttributes {
  return node.attributes;
}

function hasAttribute(node: ts.JsxOpeningLikeElement, name: string): boolean {
  return jsxAttributes(node).properties.some(
    (property) => ts.isJsxAttribute(property) && property.name.getText() === name,
  );
}

function attributeText(node: ts.JsxOpeningLikeElement, name: string): string | undefined {
  const attribute = jsxAttributes(node).properties.find(
    (property) => ts.isJsxAttribute(property) && property.name.getText() === name,
  );
  return attribute && ts.isJsxAttribute(attribute) ? attribute.initializer?.getText() : undefined;
}

function hasAncestor(node: ts.Node, componentName: string): boolean {
  let current: ts.Node | undefined = node.parent;
  while (current) {
    if (ts.isJsxElement(current) && current.openingElement.tagName.getText() === componentName) {
      return true;
    }
    current = current.parent;
  }
  return false;
}

function lineOf(tree: ts.SourceFile, node: ts.Node): number {
  return tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
}

function accessibilityViolations(file: string): string[] {
  const tree = ts.createSourceFile(
    file,
    fs.readFileSync(file, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const violations: string[] = [];
  const labelsFor = new Set<string>();

  const collectLabels = (node: ts.Node) => {
    if (
      (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) &&
      node.tagName.getText() === "label"
    ) {
      const target = attributeText(node, "htmlFor");
      if (target) labelsFor.add(target);
    }
    ts.forEachChild(node, collectLabels);
  };
  collectLabels(tree);

  const report = (node: ts.Node, message: string) => {
    violations.push(`${path.relative(process.cwd(), file)}:${lineOf(tree, node)}: ${message}`);
  };

  const visit = (node: ts.Node) => {
    if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
      const tag = node.tagName.getText();

      if (nativeControls.has(tag)) {
        const id = attributeText(node, "id");
        const explicitlyNamed =
          hasAttribute(node, "aria-label") ||
          hasAttribute(node, "aria-labelledby") ||
          hasAncestor(node, "label") ||
          hasAncestor(node, "FormField") ||
          (id !== undefined && labelsFor.has(id));

        if (!explicitlyNamed) {
          report(
            node,
            `<${tag}> has no explicit label; a placeholder is never accepted as its name`,
          );
        }
      }

      const intrinsicTag = /^[a-z]/.test(tag);
      if (intrinsicTag && hasAttribute(node, "onClick") && !nativeInteractiveElements.has(tag)) {
        report(node, `<${tag}> handles clicks without native keyboard semantics`);
      }

      if (
        ["IconButton", "MediaButton", "ReactionToggle"].includes(tag) &&
        !hasAttribute(node, "label")
      ) {
        report(node, `<${tag}> must expose its required accessible label`);
      }

      const tabIndex = attributeText(node, "tabIndex");
      if (tabIndex && /[1-9]/.test(tabIndex)) {
        report(node, "positive tabIndex overrides the product's logical DOM focus order");
      }
    }

    ts.forEachChild(node, visit);
  };
  visit(tree);
  return violations;
}

describe("product accessibility contract", () => {
  it("gives every native form control an explicit accessible name", () => {
    const violations = productFiles.flatMap(accessibilityViolations);
    expect(violations, violations.join("\n")).toEqual([]);
  });
});
