import { resolve } from "node:path";
import { defineConfig } from "vite";
import preact from "@preact/preset-vite";

export default defineConfig({
  plugins: [preact()],
  build: {
    outDir: "../hub/assets",
    emptyOutDir: false,
    sourcemap: false,
    rollupOptions: {
      input: {
        app: resolve(__dirname, "index.html"),
        login: resolve(__dirname, "src/login.ts"),
      },
      output: {
        entryFileNames: (chunk) =>
          chunk.name === "login" ? "login.js" : "app.js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: (asset) =>
          asset.names?.some((name) => name.endsWith(".css"))
            ? "app.css"
            : "assets/[name]-[hash][extname]",
      },
    },
  },
});
