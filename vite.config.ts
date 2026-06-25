import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { spawnSync } from "node:child_process";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
// 自定义插件：复制i18n locales到src-tauri
const copyI18nPlugin = () => {
  return {
    name: 'copy-i18n-locales',
    buildStart() {
      const result = spawnSync("bun", ["run", "copy-i18n"], {
        stdio: "inherit",
        shell: process.platform === "win32",
      });

      if (result.status !== 0) {
        throw new Error("Failed to copy i18n locales");
      }
    }
  };
};

export default defineConfig(async () => ({
  plugins: [vue(), copyI18nPlugin()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 12345,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
