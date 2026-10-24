import { ref } from "vue";

const workingTreeRefreshToken = ref(0);
const commitLogRefreshToken = ref(0);
const versionSwitchToken = ref(0);
const workingTreeChangeCount = ref(0);
const syncRefreshToken = ref(0);
const diskChangeToken = ref(0);

export function useProjectEvents() {
  function notifyWorkingTreeChanged() {
    workingTreeRefreshToken.value += 1;
  }

  /** Files of the open project changed on disk, outside the app (see useDiskWatch). */
  function notifyDiskChanged() {
    diskChangeToken.value += 1;
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
    diskChangeToken,
    notifyWorkingTreeChanged,
    notifyDiskChanged,
    notifySyncChanged,
    notifyVersionSaved,
    notifyVersionContextChanged,
    notifyVersionSwitched,
    setWorkingTreeChangeCount,
  };
}
