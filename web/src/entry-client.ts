import "./document.css";
import "./story-transitions.css";
import "./components/portal-entry.css";
import { hydrate } from "@solidjs/web";
import { createSiteRuntime } from "./reactivescript";
import program from "./Page.rx?program";

if (import.meta.env.DEV) {
  void import("virtual:stylex:css-only");
}

hydrate(program(createSiteRuntime()), document);
