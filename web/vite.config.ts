import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import type { Plugin } from "vite";
import { defineConfig } from "vitest/config";

const wasm = fileURLToPath(
  new URL("../crates/retiretui_wasm/pkg", import.meta.url),
);

const INLINE_SCRIPT = /<script(?![^>]*\ssrc=)[^>]*>([\s\S]*?)<\/script>/g;

function policyOf(html: string): string {
  const hashes = Array.from(html.matchAll(INLINE_SCRIPT), ([, text = ""]) => {
    const hash = createHash("sha256").update(text).digest("base64");
    return `'sha256-${hash}'`;
  });
  return [
    "default-src 'none'",
    `script-src 'self' 'wasm-unsafe-eval' ${hashes.join(" ")}`,
    // A menu or dialog locks the page's scroll with a style it writes, which no hash can name.
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self'",
    "connect-src 'self'",
    "worker-src 'self'",
    "manifest-src 'self'",
    "base-uri 'none'",
    "form-action 'self'",
  ].join("; ");
}

/** Left out of the dev server's page, which Vite writes scripts of its own into. */
const csp: Plugin = {
  name: "csp",
  apply: "build",
  transformIndexHtml: {
    order: "post",
    handler: (html) => [
      {
        tag: "meta",
        attrs: {
          "http-equiv": "Content-Security-Policy",
          content: policyOf(html),
        },
        injectTo: "head-prepend",
      },
    ],
  },
};

export default defineConfig({
  base: "./",
  plugins: [react(), tailwindcss(), csp],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
      "@wasm": wasm,
    },
  },
  server: { fs: { allow: [".", wasm] } },
  build: { target: "es2022", manifest: "manifest.json" },
  worker: { format: "es" },
  test: { include: ["src/**/*.test.ts"] },
});
