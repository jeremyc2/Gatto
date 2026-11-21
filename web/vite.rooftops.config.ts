import { defineConfig } from "vite";
import stylex from "@stylexjs/unplugin/vite";

const webRoot = new URL(".", import.meta.url).pathname;

export default defineConfig({
  root: webRoot,
  base: "/",
  publicDir: false,
  plugins: [
    stylex({
      useCSSLayers: true,
      devMode: "css-only",
      lightningcssOptions: { minify: true },
      unstable_moduleResolution: { type: "commonJS" },
    }),
  ],
  build: {
    outDir: "dist/client",
    emptyOutDir: false,
    assetsInlineLimit: 0,
    rollupOptions: {
      input: {
        rooftops: new URL("./rooftops/index.html", import.meta.url).pathname,
      },
    },
  },
});
