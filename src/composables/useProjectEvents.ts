import { ref } from "vue";

const workingTreeRefreshToken = ref(0);
const commitLogRefreshToken = ref(0);
const versionSwitchToken = ref(0);
const workingTreeChangeCount = ref(0);
const syncRefreshToken = ref(0);

export function useProjectEvents() {
  function notifyWorkingTreeChanged() {
    workingTreeRefreshToken.value += 1;
  }

  function notifyVersionSaved() {
    workingTreeRefreshToken.value += 1;
    commitLogRefreshToken.value += 1;
    syncRefreshToken.value += 1;
  }

  function notifyVersionContextChanged() {
    workingTreeRefreshToken.value += 1;
    commitLogRefreshToken.value += 1;
    syncRefreshToken.value += 1;
  }

  function notifySyncChanged() {
    syncRefreshToken.value += 1;
  }

  function notifyVersionSwitched() {
    notifyVersionContextChanged();
    versionSwitchToken.value += 1;
  }

  function setWorkingTreeChangeCount(count: number) {
    workingTreeChangeCount.value = count;
  }

  return {
    workingTreeRefreshToken,
    commitLogRefreshToken,
    versionSwitchToken,
    workingTreeChangeCount,
    syncRefreshToken,
    notifyWorkingTreeChanged,
    notifySyncChanged,
    notifyVersionSaved,
    notifyVersionContextChanged,
    notifyVersionSwitched,
    setWorkingTreeChangeCount,
  };
}
