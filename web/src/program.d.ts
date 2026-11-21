/** A plain .rx import in Vite emits a program factory, rather than its TSX projection. */
declare module "*.rx?program" {
  const program: (
    runtime: ReturnType<typeof import("@reactivescript/runtime").createApp>,
  ) => () => import("@solidjs/web").JSX.Element;
  export default program;
}
