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
          content="Gatto is a small Mac app for getting images into GitHub conversations. Bring an image, upload it to GitHub, and copy the Markdown."
        />
        <meta name="color-scheme" content="light dark" />
        <title>Gatto — Your image. Ready to share.</title>
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
