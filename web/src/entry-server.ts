import { renderToStream } from "@solidjs/web";
import type { StartServerRenderContext } from "@solidjs/vite-plugin";
import manifest from "virtual:solid-manifest";
import { createSiteRuntime } from "./reactivescript";
import program from "./Page.rx?program";

/** Start mode owns the request; each render gets a fresh ReactiveScript program. */
export function render(request: Request, context: StartServerRenderContext) {
  return renderToStream(program(createSiteRuntime()), {
    manifest,
    ...(context.nonce == null ? {} : { nonce: context.nonce }),
    signal: request.signal,
  });
}
