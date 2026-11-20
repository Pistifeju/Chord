// Vite config for the variants page: its own port and cache, so the app's dev server keeps port 1420.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  cacheDir: "node_modules/.vite-variants",
  optimizeDeps: { entries: [".impeccable/mockups/*/index.html"] },
  server: {
    port: 1421,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
  },
});
