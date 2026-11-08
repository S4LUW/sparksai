import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Porta fixa porque o Tauri abre a janela em devUrl (src-tauri/tauri.conf.json).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
