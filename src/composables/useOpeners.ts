import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n, type TranslationKey } from "./useI18n";
import { useToast } from "./useToast";
import { errorText } from "../utils/errors";
import type { OpenerView } from "../types";

/** Shared across the toolbar menu, the folder menu and the manage dialog. */
const openers = ref<OpenerView[]>([]);

const KIND_ORDER = { terminal: 0, editor: 1, files: 2 } as const;

/** Opening folders in terminals, editors and the file manager, and managing that list. */
export function useOpeners() {
  const { t } = useI18n();
  const toast = useToast();

  const defaultOpener = computed(() => openers.value.find((opener) => opener.default) ?? null);

  /** What the menu offers, in groups with a line between them: the default alone first, then one
   *  group per kind. Hidden ones are left out. */
  const menuGroups = computed(() => {
    const shown = openers.value
      .filter((opener) => !opener.hidden)
      .sort((a, b) => Number(b.default) - Number(a.default) || KIND_ORDER[a.kind] - KIND_ORDER[b.kind]);
    const groups: OpenerView[][] = [];
    for (const opener of shown) {
      const last = groups[groups.length - 1];
      if (last && !last[0].default && last[0].kind === opener.kind) last.push(opener);
      else groups.push([opener]);
    }
    return groups;
  });

  /** Everything, hidden ones included, under a heading per kind and the user's own last. */
  const manageSections = computed(() => {
    const sections: { title: TranslationKey; items: OpenerView[] }[] = [
      { title: "openerTerminal", items: openers.value.filter((o) => !o.custom && o.kind === "terminal") },
      { title: "openerEditor", items: openers.value.filter((o) => !o.custom && o.kind === "editor") },
      { title: "openerFiles", items: openers.value.filter((o) => !o.custom && o.kind === "files") },
      { title: "openerCustom", items: openers.value.filter((o) => o.custom) },
    ];
    return sections.filter((section) => section.items.length > 0);
  });

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
    defaultOpener,
    menuGroups,
    manageSections,
    load,
    openFolder,
    setDefault: (id: string) => change("set_default_opener", { id }),
    setHidden: (id: string, hidden: boolean) => change("set_opener_hidden", { id, hidden }),
    addCustom: (name: string, program: string, args: string) => change("add_custom_opener", { name, program, args }),
    removeCustom: (id: string) => change("remove_custom_opener", { id }),
  };
}
