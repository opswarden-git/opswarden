import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

function sourceFiles(directory: string): string[] {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) return sourceFiles(target);
    return entry.name.endsWith(".tsx") && !entry.name.endsWith(".test.tsx") ? [target] : [];
  });
}

type Confirmation = {
  file: string;
  intent?: string;
  attributes: Set<string>;
};

function confirmations(file: string): Confirmation[] {
  const tree = ts.createSourceFile(
    file,
    fs.readFileSync(file, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const result: Confirmation[] = [];

  const visit = (node: ts.Node) => {
    if (
      (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) &&
      node.tagName.getText() === "ConfirmDialog"
    ) {
      const attributes = node.attributes.properties.filter(ts.isJsxAttribute);
      const intent = attributes.find((attribute) => attribute.name.getText() === "intent");
      result.push({
        file: path.relative(process.cwd(), file),
        intent:
          intent?.initializer && ts.isStringLiteral(intent.initializer)
            ? intent.initializer.text
            : undefined,
        attributes: new Set(attributes.map((attribute) => attribute.name.getText())),
      });
    }
    ts.forEachChild(node, visit);
  };
  visit(tree);
  return result;
}

describe("destructive-action disclosure contract", () => {
  const productConfirmations = sourceFiles(path.join(process.cwd(), "components")).flatMap(
    confirmations,
  );

  it("makes every confirmation explicit, named and cancellable", () => {
    expect(productConfirmations).toHaveLength(11);
    for (const confirmation of productConfirmations) {
      for (const required of [
        "title",
        "description",
        "confirmLabel",
        "cancelLabel",
        "intent",
        "onConfirm",
        "onClose",
      ]) {
        expect(confirmation.attributes.has(required), `${confirmation.file}: ${required}`).toBe(
          true,
        );
      }
    }
  });

  it("declares the risk of every flow instead of inferring it from wording or color", () => {
    expect(productConfirmations.filter(({ intent }) => intent === "destructive")).toHaveLength(9);
    expect(productConfirmations.filter(({ intent }) => intent === "standard")).toHaveLength(2);
    expect(productConfirmations.some(({ intent }) => intent === undefined)).toBe(false);
  });

  it("adds typed confirmation to the three irreversible aggregate deletions", () => {
    expect(
      productConfirmations.filter(({ attributes }) => attributes.has("requireType")),
    ).toHaveLength(3);
  });
});
