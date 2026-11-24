import { readFile } from "node:fs/promises";
import { defineConfig, type Plugin } from "vite";
import solid from "@solidjs/vite-plugin";
import stylex from "@stylexjs/unplugin/vite";

const webRoot = new URL(".", import.meta.url).pathname;
const repoRoot = new URL("..", import.meta.url).pathname;
const rooftopsHtmlPath = new URL("./rooftops/index.html", import.meta.url).pathname;

const rooftopsDistHtmlPath = new URL("./dist/client/rooftops/index.html", import.meta.url).pathname;

function rooftopsRoute(): Plugin {
  return {
    name: "gatto-rooftops-route",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const pathname = new URL(req.url ?? "/", "http://localhost").pathname;
        if (pathname === "/rooftops") {
          res.statusCode = 302;
          res.setHeader("Location", "/rooftops/");
          res.end();
          return;
        }
        if (pathname !== "/rooftops/" && pathname !== "/rooftops/index.html") {
          next();
          return;
        }
        void (async () => {
          const source = await readFile(rooftopsHtmlPath, "utf8");
          const withStyleX = source.replace(
            "</head>",
            '    <link rel="stylesheet" href="/virtual:stylex.css" />\n  </head>',
          );
          const html = await server.transformIndexHtml(req.url ?? "/rooftops/", withStyleX);
          res.statusCode = 200;
          res.setHeader("Content-Type", "text/html; charset=utf-8");
          res.end(html);
        })().catch(next);
      });
    },
    configurePreviewServer(server) {
      server.middlewares.use((req, res, next) => {
        const pathname = new URL(req.url ?? "/", "http://localhost").pathname;
        if (pathname === "/rooftops") {
          res.statusCode = 302;
          res.setHeader("Location", "/rooftops/");
          res.end();
          return;
        }
        if (pathname !== "/rooftops/" && pathname !== "/rooftops/index.html") {
          next();
          return;
        }
        void (async () => {
          const html = await readFile(rooftopsDistHtmlPath, "utf8");
          res.statusCode = 200;
          res.setHeader("Content-Type", "text/html; charset=utf-8");
          res.end(html);
        })().catch(next);
      });
    },
  };
}

export default defineConfig(({ command }) => ({
  root: webRoot,
  plugins: [
    rooftopsRoute(),
    stylex({
      useCSSLayers: true,
      devMode: "css-only",
      lightningcssOptions: { minify: command === "build" },
      unstable_moduleResolution: { type: "commonJS" },
    }),
    solid({ start: true, ssr: true }),
  ],
  server: { host: "127.0.0.1", port: 3000, strictPort: true, fs: { allow: [repoRoot] } },
  preview: { host: "127.0.0.1", port: 3001, strictPort: true },
  build: { assetsInlineLimit: 0 },
}));
