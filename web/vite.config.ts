import { defineConfig } from "vite";
import solid from "@solidjs/vite-plugin";
import stylex from "@stylexjs/unplugin/vite";

export default defineConfig(({ command }) => ({
  root: new URL(".", import.meta.url).pathname,
  plugins: [
    stylex({
      useCSSLayers: true,
      devMode: "css-only",
      lightningcssOptions: { minify: command === "build" },
      unstable_moduleResolution: { type: "commonJS" },
    }),
    solid({ start: true, ssr: true }),
  ],
  server: { host: "127.0.0.1", port: 3000, strictPort: true },
  preview: { host: "127.0.0.1", port: 3001, strictPort: true },
  build: { assetsInlineLimit: 0 },
}));
