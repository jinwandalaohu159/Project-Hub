import { flushPromises } from "@vue/test-utils";
import { ref } from "vue";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const invoke = vi.fn(async (_command: string, _args?: unknown) => undefined);
vi.mock("@tauri-apps/api/core", () => ({ invoke: (command: string, args: unknown) => invoke(command, args) }));

let emit: ((payload: string) => void) | null = null;
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (_event: string, handler: (event: { payload: string }) => void) => {
    emit = (payload) => handler({ payload });
    return () => (emit = null);
  },
}));

const selectedProject = ref<{ path: string; missing: boolean } | null>(null);
vi.mock("../composables/useProjectStore", () => ({ useProjectStore: () => ({ selectedProject }) }));

import { useDiskWatch } from "../composables/useDiskWatch";
import { useProjectEvents } from "../composables/useProjectEvents";

const project = (path: string) => ({ path, missing: false });
const calls = (command: string) => invoke.mock.calls.filter(([name]) => name === command).map(([, args]) => args);

describe("following the open project on disk", () => {
  const { diskChangeToken, workingTreeRefreshToken } = useProjectEvents();
  const diskWatch = useDiskWatch();

  beforeEach(async () => {
    invoke.mockClear();
    selectedProject.value = project("/work/a");
    await diskWatch.start();
    await flushPromises();
  });

  afterEach(async () => {
    diskWatch.stop();
    await flushPromises();
  });

  it("watches the open project and tells the views when its files change", async () => {
    expect(calls("watch_worktree")).toEqual([{ path: "/work/a" }]);
    expect(diskWatch.watching.value).toBe(true);

    const before = diskChangeToken.value;
    const workingTreeBefore = workingTreeRefreshToken.value;
    emit?.("/work/other");
    expect(diskChangeToken.value).toBe(before);
    emit?.("/work/a");
    expect(diskChangeToken.value).toBe(before + 1);
    expect(workingTreeRefreshToken.value).toBe(workingTreeBefore + 1);
  });

  it("asks the app in order during a quick switch, and stops with the app", async () => {
    let finishB!: () => void;
    invoke.mockImplementationOnce(() => new Promise<undefined>((resolve) => (finishB = () => resolve(undefined))));
    selectedProject.value = project("/work/b");
    await flushPromises();
    selectedProject.value = project("/work/c");
    await flushPromises();
    // c waits for b's request, so the app ends up watching c.
    expect(calls("watch_worktree")).toEqual([{ path: "/work/a" }, { path: "/work/b" }]);
    finishB();
    await flushPromises();
    expect(calls("watch_worktree")).toEqual([{ path: "/work/a" }, { path: "/work/b" }, { path: "/work/c" }]);

    const before = diskChangeToken.value;
    emit?.("/work/b");
    expect(diskChangeToken.value).toBe(before);
    emit?.("/work/c");
    expect(diskChangeToken.value).toBe(before + 1);

    diskWatch.stop();
    await flushPromises();
    expect(invoke).toHaveBeenLastCalledWith("unwatch_worktree", { path: "/work/c" });
    expect(diskWatch.watching.value).toBe(false);
  });

  it("looks again when the window comes back", () => {
    const before = diskChangeToken.value;
    window.dispatchEvent(new Event("focus"));
    expect(diskChangeToken.value).toBe(before + 1);
  });

  it("reports not watching where the app cannot watch", async () => {
    invoke.mockImplementationOnce(async () => {
      throw new Error("unavailable");
    });
    selectedProject.value = project("/work/d");
    await flushPromises();
    expect(diskWatch.watching.value).toBe(false);
  });
});
