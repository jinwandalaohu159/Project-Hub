import { configDefaults, defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "happy-dom",
    // Scratch copies of other projects must never be picked up as this app's tests.
    exclude: [...configDefaults.exclude, ".claude/**", "backup/**", "release/**"],
  },
});
