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
        <title>Gatto — GitHub, illustrated.</title>
        <link rel="icon" href="/favicon.ico" sizes="16x16 32x32 48x48" />
        <link rel="icon" href="/favicon-96.png" type="image/png" sizes="96x96" />
        <link rel="icon" href="/favicon.svg" type="image/svg+xml" sizes="any" />
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
