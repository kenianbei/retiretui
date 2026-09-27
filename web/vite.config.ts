import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const wasm = fileURLToPath(
  new URL("../crates/retiretui_wasm/pkg", import.meta.url),
);

export default defineConfig({
  base: "./",
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
      "@wasm": wasm,
    },
  },
  server: { fs: { allow: [".", wasm, "../crates/retiretui_wasm/bindings"] } },
  worker: { format: "es" },
  test: { include: ["src/**/*.test.ts"] },
});
