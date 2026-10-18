import { getRequestEvent } from "@solidjs/web";

/** The same path decision is used for static rendering and hydration. */
export function isGattoverse() {
  const path = import.meta.env.SSR
    ? new URL(getRequestEvent()?.request.url ?? "http://localhost/").pathname
    : window.location.pathname;
  return path === "/gattoverse" || path === "/gattoverse/";
}
