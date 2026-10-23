import { createApp } from "vue";
import App from "./App.vue";
import UpdateHelperWindow from "./components/update/UpdateHelperWindow.vue";
import { logApp } from "./composables/useAppLog";
import "./style.css";

function describe(error: unknown) {
  if (error instanceof Error) return error.stack || `${error.name}: ${error.message}`;
  return String(error);
}

declare global {
  interface Window {
    /** Set by the update helper process for its progress window (see self_update.rs). */
    __PROJECT_HUB_UPDATER__?: boolean;
  }
}

const app = createApp(window.__PROJECT_HUB_UPDATER__ ? UpdateHelperWindow : App);
// Unexpected UI errors are kept in the app log; the console still shows them during development.
app.config.errorHandler = (error, _instance, info) => {
  console.error(error);
  void logApp("error", "ui.exception", `${describe(error)} (${info})`);
};
window.addEventListener("error", (event) => {
  void logApp("error", "ui.exception", describe(event.error ?? event.message));
});
window.addEventListener("unhandledrejection", (event) => {
  void logApp("error", "ui.exception", describe(event.reason));
});
app.mount("#app");
