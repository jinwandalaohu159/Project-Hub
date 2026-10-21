import { describe, it, expect } from "vitest";
import { useI18n } from "../composables/useI18n";
import { formatRelativeTime } from "../utils/time";
import {
  basename,
  changeDisplayPath,
  dirname,
  isChildPath,
  joinPath,
  pathDepth,
  replacePathPrefix,
  samePath,
} from "../utils/path";
import { findNode, removeNode, sortNodes, updateNodePath } from "../utils/fileTree";
import { isTauri } from "../utils/env";
import { createProjectCache } from "../utils/projectCache";
import { useProjectData } from "../composables/useProjectData";

describe("formatRelativeTime", () => {
  const { t, setLocale } = useI18n();
  const now = new Date("2026-10-20T12:00:00Z").getTime();
  const ago = (ms: number) => new Date(now - ms).toISOString();

  it("covers every range", () => {
    setLocale("zh");
    expect(formatRelativeTime(null, t, now)).toBe("");
    expect(formatRelativeTime(ago(10_000), t, now)).toBe("刚刚");
    expect(formatRelativeTime(ago(5 * 60_000), t, now)).toBe("5分钟前");
    expect(formatRelativeTime(ago(3 * 3_600_000), t, now)).toBe("3小时前");
    expect(formatRelativeTime(ago(2 * 86_400_000), t, now)).toBe("2天前");
    expect(formatRelativeTime(ago(40 * 86_400_000), t, now)).toBe(new Date(now - 40 * 86_400_000).toLocaleDateString());
  });
});

describe("path helpers", () => {
  it("handle both separators", () => {
    expect(joinPath("C:\\a", "b")).toBe("C:\\a/b");
    expect(joinPath("C:\\a\\", "b")).toBe("C:\\a\\b");
    expect(dirname("C:\\a\\b.txt")).toBe("C:\\a");
    expect(dirname("/a/b")).toBe("/a");
    expect(basename("C:/a/b.txt")).toBe("b.txt");
    expect(isChildPath("C:\\a\\b", "C:\\a")).toBe(true);
    expect(isChildPath("C:/ab", "C:/a")).toBe(false);
    expect(pathDepth("C:\\a/b\\c")).toBe(4);
    expect(samePath("C:\\a\\b", "C:/a/b")).toBe(true);
  });

  it("rewrites prefixes only on path boundaries", () => {
    expect(replacePathPrefix("/p/old", "/p/old", "/p/new")).toBe("/p/new");
    expect(replacePathPrefix("/p/old/x.txt", "/p/old", "/p/new")).toBe("/p/new/x.txt");
    expect(replacePathPrefix("/p/older", "/p/old", "/p/new")).toBe("/p/older");
  });

  it("formats renamed changes", () => {
    expect(changeDisplayPath({ path: "b.txt", old_path: "a.txt" })).toBe("a.txt → b.txt");
    expect(changeDisplayPath({ path: "b.txt", old_path: null })).toBe("b.txt");
  });
});

describe("file tree helpers", () => {
  type Node = { name: string; path: string; is_dir: boolean; children: Node[] };
  const tree = (): Node[] => [
    { name: "src", path: "/r/src", is_dir: true, children: [{ name: "a.ts", path: "/r/src/a.ts", is_dir: false, children: [] }] },
    { name: "z.md", path: "/r/z.md", is_dir: false, children: [] },
  ];

  it("finds and removes nested nodes", () => {
    const nodes = tree();
    expect(findNode(nodes, "/r/src/a.ts")?.name).toBe("a.ts");
    expect(removeNode(nodes, "/r/src/a.ts")?.name).toBe("a.ts");
    expect(findNode(nodes, "/r/src/a.ts")).toBeNull();
    expect(removeNode(nodes, "/r/missing")).toBeNull();
  });

  it("renames a subtree and keeps folders first", () => {
    const nodes = tree();
    updateNodePath(nodes[0], "/r/src", "/r/lib");
    expect(nodes[0].path).toBe("/r/lib");
    expect(nodes[0].children[0].path).toBe("/r/lib/a.ts");
    const mixed: Node[] = [nodes[1], { name: "b", path: "/r/b", is_dir: true, children: [] }, { name: "a.md", path: "/r/a.md", is_dir: false, children: [] }];
    sortNodes(mixed);
    expect(mixed.map((node) => node.name)).toEqual(["b", "a.md", "z.md"]);
  });
});

describe("isTauri", () => {
  it("is false outside the desktop shell", () => {
    expect(isTauri()).toBe(false);
  });
});

describe("createProjectCache", () => {
  it("keeps the most recently used projects within its limit", () => {
    const cache = createProjectCache<number>(2);
    cache.set("a", 1);
    cache.set("b", 2);
    expect(cache.get("a")).toBe(1); // "a" is now the most recent
    cache.set("c", 3);
    expect(cache.get("b")).toBeUndefined();
    expect(cache.get("a")).toBe(1);
    expect(cache.get("c")).toBe(3);
  });

  it("replaces a project's value in place", () => {
    const cache = createProjectCache<string[]>();
    cache.set("a", ["old"]);
    cache.set("a", ["new"]);
    expect(cache.get("a")).toEqual(["new"]);
  });
});

describe("useProjectData", () => {
  it("shows a project's last value at once and keeps fresh ones", () => {
    const records = useProjectData<string[]>("test-records", () => []);
    expect(records.restore("a")).toBe(false);
    records.update("a", ["a1"]);
    records.update("b", ["b1"]);
    expect(records.data.value).toEqual(["b1"]);

    expect(records.restore("a")).toBe(true);
    expect(records.data.value).toEqual(["a1"]);
    expect(records.projectId.value).toBe("a");

    // Shown but not kept: switching back brings the last kept value.
    records.show("a", ["edited"]);
    records.restore("b");
    records.restore("a");
    expect(records.data.value).toEqual(["a1"]);

    records.clear();
    expect(records.data.value).toEqual([]);
    expect(records.projectId.value).toBeNull();
  });

  it("lets a panel created again find what the previous one loaded", () => {
    useProjectData<number>("test-shared", () => 0).update("a", 7);
    const again = useProjectData<number>("test-shared", () => 0);
    expect(again.data.value).toBe(0);
    expect(again.restore("a")).toBe(true);
    expect(again.data.value).toBe(7);
  });
});
