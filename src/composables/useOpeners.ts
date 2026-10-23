import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n, type TranslationKey } from "./useI18n";
import { useToast } from "./useToast";
import { errorText } from "../utils/errors";
import type { OpenerView } from "../types";

/** Shared by the toolbar menu and the manage dialog. */
const openers = ref<OpenerView[]>([]);

/** The groups, in the order they are listed. */
const KINDS: { kind: OpenerView["kind"]; title: TranslationKey }[] = [
  { kind: "terminal", title: "openerTerminal" },
  { kind: "editor", title: "openerEditor" },
  { kind: "files", title: "openerFiles" },
  { kind: "other", title: "openerOther" },
];

/** `list` under a heading per kind, leaving out kinds with nothing in them. */
function grouped(list: OpenerView[]) {
  return KINDS.map(({ kind, title }) => ({ kind, title, items: list.filter((opener) => opener.kind === kind) })).filter(
    (group) => group.items.length > 0,
  );
}

/** Opening folders in terminals, editors and the file manager, and managing that list. */
export function useOpeners() {
  const { t } = useI18n();
  const toast = useToast();

  /** What the menu offers: one group per kind, hidden ones left out. */
  const menuGroups = computed(() => grouped(openers.value.filter((opener) => !opener.hidden)));

  /** Everything, hidden ones included, grouped the same way. */
  const manageSections = computed(() => grouped(openers.value));

  /** Looks the programs up again; they may have been installed or removed meanwhile. */
  async function load() {
    openers.value = await invoke<OpenerView[]>("list_openers");
  }

  async function openFolder(opener: OpenerView, path: string) {
    try {
      await invoke("open_folder_with", { id: opener.id, path });
    } catch (error) {
      toast.error(errorText(error, t), { detail: t("openWithIn", { name: opener.name }) });
    }
  }

  /** Applies a change to the list; false when it failed (already reported). */
  async function change(command: string, args: Record<string, unknown>) {
    try {
      openers.value = await invoke<OpenerView[]>(command, args);
      return true;
    } catch (error) {
      toast.error(errorText(error, t));
      return false;
    }
  }

  return {
    openers,
    menuGroups,
    manageSections,
    load,
    openFolder,
    setHidden: (id: string, hidden: boolean) => change("set_opener_hidden", { id, hidden }),
    addCustom: (name: string, program: string, args: string) => change("add_custom_opener", { name, program, args }),
    removeCustom: (id: string) => change("remove_custom_opener", { id }),
  };
}
