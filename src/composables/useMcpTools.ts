import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../utils/env";
import type { McpTool } from "../types";

// Loaded once: the tools only change with a new version of the app.
const tools = ref<McpTool[]>([]);
let loading: Promise<void> | null = null;

/** The tools agents get from the MCP server, straight from the backend's definitions. */
export function useMcpTools() {
  function load() {
    if (!isTauri()) return Promise.resolve();
    loading ??= invoke<McpTool[]>("get_mcp_tools")
      .then((list) => {
        tools.value = list;
      })
      .catch(() => {
        loading = null;
      });
    return loading;
  }

  /** Runs a read-only tool exactly as an agent would; the backend refuses any other. */
  function tryTool(name: string, args: Record<string, unknown>) {
    return invoke<string>("try_mcp_tool", { name, args });
  }

  return { tools, load, tryTool };
}

export function isReadOnly(tool: McpTool) {
  return tool.annotations?.readOnlyHint === true;
}
