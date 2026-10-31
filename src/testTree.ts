import type { TestItem } from "./host/client";

/** The FQN is the only source: project > prefix > class > method. Nothing is parsed from
 * source files, so a name with no dot still lands in honest placeholder groups. */
export type TestNameParts = { prefix: string; className: string; method: string };

export function splitTestName(fullyQualifiedName: string): TestNameParts {
  const call = fullyQualifiedName.indexOf("(");
  const head = call >= 0 ? fullyQualifiedName.slice(0, call) : fullyQualifiedName;
  const parts = head.split(".").filter((part) => part.length > 0);
  if (parts.length === 0) {
    return { prefix: "(no namespace)", className: "(no class)", method: fullyQualifiedName };
  }
  if (parts.length === 1) {
    return { prefix: "(no namespace)", className: "(no class)", method: parts[0] };
  }
  return {
    prefix: parts.length > 2 ? parts.slice(0, -2).join(".") : "(no namespace)",
    className: parts[parts.length - 2],
    method: parts[parts.length - 1],
  };
}

export type TestLeaf = {
  kind: "test";
  key: string;
  /** Index in the discovery list, so test-item-N stays the same identity across grouping. */
  index: number;
  item: TestItem;
  parts: TestNameParts;
};

export type TestGroup = {
  kind: "group";
  key: string;
  level: "project" | "prefix" | "class";
  title: string;
  tooltip: string;
  children: TreeNode[];
};

export type TreeNode = TestGroup | TestLeaf;

function projectTitle(path: string) {
  const file = path.split("/").pop() ?? path;
  return file.replace(/\.csproj$/i, "") || path;
}

function childGroup(
  siblings: TreeNode[],
  key: string,
  level: TestGroup["level"],
  title: string,
  tooltip: string,
) {
  const existing = siblings.find(
    (node): node is TestGroup => node.kind === "group" && node.key === key,
  );
  if (existing) return existing;
  const group: TestGroup = { kind: "group", key, level, title, tooltip, children: [] };
  siblings.push(group);
  return group;
}

/** Groups in discovery order; each level keeps the order in which its first test appeared. */
export function buildTestTree(items: TestItem[]): TestGroup[] {
  const roots: TestGroup[] = [];
  items.forEach((item, index) => {
    const parts = splitTestName(item.fullyQualifiedName);
    const project = childGroup(
      roots,
      `project|${item.project}`,
      "project",
      projectTitle(item.project),
      `Project: ${item.project}`,
    );
    const prefix = childGroup(
      project.children,
      `prefix|${item.project}|${parts.prefix}`,
      "prefix",
      parts.prefix,
      `Namespace prefix derived from the fully qualified test name: ${item.fullyQualifiedName}`,
    );
    const klass = childGroup(
      prefix.children,
      `class|${item.project}|${parts.prefix}|${parts.className}`,
      "class",
      parts.className,
      `Class derived from the fully qualified test name: ${item.fullyQualifiedName}`,
    );
    klass.children.push({
      kind: "test",
      key: `${item.discoveryId}:${item.id}:${index}`,
      index,
      item,
      parts,
    });
  });
  return roots;
}

export function testCount(node: TreeNode): number {
  if (node.kind === "test") return 1;
  return node.children.reduce((total, child) => total + testCount(child), 0);
}

/** Drops non-matching leaves and the groups left empty, keeping the groups of matches. */
export function filterTestTree(groups: TestGroup[], needle: string): TestGroup[] {
  const query = needle.trim().toLowerCase();
  if (!query) return groups;
  const prune = (nodes: TreeNode[]): TreeNode[] => {
    const kept: TreeNode[] = [];
    for (const node of nodes) {
      if (node.kind === "test") {
        const haystack = `${node.item.fullyQualifiedName}\n${node.item.displayName}`.toLowerCase();
        if (haystack.includes(query)) kept.push(node);
      } else {
        const children = prune(node.children);
        if (children.length > 0) kept.push({ ...node, children });
      }
    }
    return kept;
  };
  return prune(groups) as TestGroup[];
}
