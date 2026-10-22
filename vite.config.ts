import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

import { resolve } from "node:path";
import { defineConfig } from "vite";

const uiRoot = resolve(import.meta.dirname, "ui");

const pages = ["index", "variation", "hud", "overlay", "tray-menu"];

export default defineConfig({
  root: uiRoot,
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    outDir: resolve(import.meta.dirname, "dist"),
    emptyOutDir: true,
    target: "chrome120",
    rollupOptions: {
      input: Object.fromEntries(pages.map((p) => [p, resolve(uiRoot, `${p}.html`)])),
    },
  },
});
