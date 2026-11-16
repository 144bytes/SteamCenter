import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The Tauri window loads the built files from the exe: no local server and no network while the app runs.
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2020",
  },
});
