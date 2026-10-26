import { HydrationScript, type JSX } from "@solidjs/web";
import * as stylex from "@stylexjs/stylex";
import { documentStyles } from "./design-system/document.stylex";
import "./document.css";
import coconat from "../assets/fonts/Coconat.woff2";
import apfel from "../assets/fonts/Apfel.woff2";

export default function Document(props: { children: JSX.Element }) {
  return (
    <html lang="en" {...stylex.attrs(documentStyles.root)}>
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta
          name="description"
          content="Gatto is a small Mac app for adding images and videos to GitHub pull requests, issues, and comments."
        />
        <meta name="color-scheme" content="light dark" />
        <meta property="og:type" content="website" />
        <meta property="og:url" content="https://gatto.jeremyc2.workers.dev/" />
        <meta property="og:title" content="Gatto — GitHub, illustrated." />
        <meta
          property="og:description"
          content="A small Mac app for adding images and videos to GitHub pull requests, issues, and comments."
        />
        <meta property="og:image" content="https://gatto.jeremyc2.workers.dev/social-image.png" />
        <meta property="og:image:type" content="image/png" />
        <meta property="og:image:width" content="1200" />
        <meta property="og:image:height" content="630" />
        <meta property="og:image:alt" content="Gatto cat icon on warm paper." />
        <meta name="twitter:card" content="summary" />
        <meta name="twitter:title" content="Gatto — GitHub, illustrated." />
        <meta
          name="twitter:description"
          content="A small Mac app for adding images and videos to GitHub pull requests, issues, and comments."
        />
        <meta name="twitter:image" content="https://gatto.jeremyc2.workers.dev/social-image.png" />
        <title>Gatto — GitHub, illustrated.</title>
        <link rel="icon" href="/favicon.ico" sizes="16x16 32x32 64x64" />
        <link rel="icon" href="/favicon-16.png" type="image/png" sizes="16x16" />
        <link rel="icon" href="/favicon-32.png" type="image/png" sizes="32x32" />
        <link rel="icon" href="/favicon-96.png" type="image/png" sizes="96x96" />
        <link rel="apple-touch-icon" href="/apple-touch-icon.png" sizes="180x180" />
        <link rel="preload" href={coconat} as="font" type="font/woff2" crossorigin="anonymous" />
        <link rel="preload" href={apfel} as="font" type="font/woff2" crossorigin="anonymous" />
        <HydrationScript />
        {import.meta.env.DEV && <link rel="stylesheet" href="/virtual:stylex.css" />}
      </head>
      <body id="top" {...stylex.attrs(documentStyles.body)}>
        {props.children}
      </body>
    </html>
  );
}
