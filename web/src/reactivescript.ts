import { createApp } from "@reactivescript/runtime";

/** Use Solid's renderer for children, including arrays, entities and absent values. */
export function createSiteRuntime() {
  const runtime = createApp({ root: undefined });
  // ReactiveScript 0.3.1's scenario-oriented show() JSON-formats all-text arrays
  // and escapes strings before Solid escapes them a second time during SSR.
  // Forward actual JSX values at this single boundary on both server and client.
  runtime.show = (value) => value;
  return runtime;
}
