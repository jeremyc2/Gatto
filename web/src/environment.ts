import { getRequestEvent } from "@solidjs/web";

/** Build-time environment values shared with the ReactiveScript document. */
export const development = import.meta.env.DEV;

/** Seed the server-rendered script nonce; its native ref captures it during hydration. */
export function scriptNonce() {
  const nonce = getRequestEvent()?.nonce;
  const script = typeof nonce === "string" ? nonce : nonce?.script;
  return typeof script === "string" ? script : undefined;
}
