import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

/**
 * VIGIL requires that "color is never the only signal — every state (Incident
 * state, severity level) is conveyed by color and icon and text, to remain
 * readable for color-blind users".
 *
 * The four chips satisfy that today. This locks it: a redesign that drops an
 * icon to tighten a row, or replaces a label with a bare dot, fails here rather
 * than in front of a jury.
 *
 * Parsed through the TypeScript AST rather than matched by regex — a pattern
 * that looks right will still miss `<CheckCircle2 />`, because component names
 * carry digits.
 */

function chipFiles(directory: string): string[] {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) return chipFiles(target);
    return entry.name.endsWith("Chip.tsx") && !entry.name.endsWith(".test.tsx") ? [target] : [];
  });
}

type Branch = {
  file: string;
  state: string;
  icons: string[];
  translated: boolean;
};

/** One entry per `case "state":` that returns JSX, with what that JSX renders. */
function stateBranches(file: string): Branch[] {
  const tree = ts.createSourceFile(
    file,
    fs.readFileSync(file, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const relative = path.relative(process.cwd(), file);
  const branches: Branch[] = [];

  const visit = (node: ts.Node) => {
    if (ts.isCaseClause(node) && ts.isStringLiteral(node.expression)) {
      const icons: string[] = [];
      let translated = false;

      const inspect = (child: ts.Node) => {
        // An icon is a JSX element whose tag is a component, not an intrinsic
        // tag: `<Flame />` counts, `<span>` does not.
        if (ts.isJsxSelfClosingElement(child) || ts.isJsxOpeningElement(child)) {
          const tag = child.tagName.getText();
          if (/^[A-Z]/.test(tag)) icons.push(tag);
        }
        // A translated label is a `t("…")` call, never a bare string.
        if (
          ts.isCallExpression(child) &&
          ts.isIdentifier(child.expression) &&
          child.expression.text === "t"
        ) {
          translated = true;
        }
        ts.forEachChild(child, inspect);
      };
      node.statements.forEach(inspect);

      if (icons.length > 0 || translated) {
        branches.push({ file: relative, state: node.expression.text, icons, translated });
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(tree);
  return branches;
}

describe("state encoding contract", () => {
  const files = chipFiles(path.join(process.cwd(), "components")).sort();
  const branches = files.flatMap(stateBranches);

  it("routes every product state through a dedicated chip", () => {
    // Compared by name, not by path: moving a chip between directories is a
    // refactor, dropping one is a regression.
    expect(files.map((file) => path.basename(file)).sort()).toEqual([
      "ReleaseStateChip.tsx",
      "RoleChip.tsx",
      "SeverityChip.tsx",
      "StateChip.tsx",
    ]);
  });

  it("covers the four severities, four incident states and five release states", () => {
    // Exact basename: `endsWith("StateChip.tsx")` would also match
    // ReleaseStateChip.tsx and silently merge two chips into one assertion.
    const states = (name: string) =>
      branches
        .filter((branch) => path.basename(branch.file) === name)
        .map((branch) => branch.state);

    expect(states("SeverityChip.tsx")).toEqual(["low", "medium", "high", "critical"]);
    expect(states("StateChip.tsx")).toEqual(["open", "acknowledged", "escalated", "resolved"]);
    expect(states("ReleaseStateChip.tsx")).toEqual([
      "created",
      "in_progress",
      "blocked",
      "completed",
      "cancelled",
    ]);
  });

  it("never lets color carry a state on its own", () => {
    expect(branches.length).toBeGreaterThan(0);
    for (const branch of branches) {
      expect(
        branch.icons.length,
        `${branch.file} — "${branch.state}" renders no icon`,
      ).toBeGreaterThan(0);
      expect(
        branch.translated,
        `${branch.file} — "${branch.state}" renders no translated label`,
      ).toBe(true);
    }
  });

  it("gives each state within a chip its own icon", () => {
    for (const file of new Set(branches.map((branch) => branch.file))) {
      const chosen = branches
        .filter((branch) => branch.file === file)
        .map((branch) => branch.icons[0]);
      expect(new Set(chosen).size, `${file} reuses an icon across states`).toBe(chosen.length);
    }
  });
});
