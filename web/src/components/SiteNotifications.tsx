import * as stylex from "@stylexjs/stylex";
import { createEffect, onCleanup } from "solid-js";
import Toastify from "toastify-js";
import { colors, media, radius, type } from "../design-system/tokens.stylex";

/** A single polite notice; repeated activations never build a stack. */
export function createSiteNotifications() {
  let region: HTMLDivElement | undefined;
  let active: ReturnType<typeof Toastify> | undefined;

  function show(message: string) {
    if (!region || active) return;
    const trigger = document.activeElement;
    if ("showPopover" in region && !region.matches(":popover-open")) region.showPopover();

    const toast = Toastify({
      text: message,
      selector: region,
      duration: 6000,
      close: true,
      gravity: "top",
      ariaLive: "off", // The persistent region announces inserted text.
      className: stylex.attrs(styles.notice).class,
      stopOnFocus: true,
      callback: () => {
        if (active === toast) active = undefined;
      },
    });
    active = toast;
    toast.showToast();

    const element = toast.toastElement;
    element?.removeAttribute("aria-live");
    const close = element?.querySelector("button");
    if (close) {
      close.className += ` ${stylex.attrs(styles.close).class}`;
      close.setAttribute("aria-label", "Dismiss notification");
      close.setAttribute("popovertarget", "site-notifications");
      close.setAttribute("popovertargetaction", "hide");
      close.addEventListener(
        "click",
        () => {
          if (trigger instanceof HTMLElement && trigger.isConnected) {
            trigger.focus({ preventScroll: true });
          }
        },
        { capture: true },
      );
    }
    // Toastify pauses on hover. Give keyboard focus the same timer behavior.
    element?.addEventListener("focusin", () => element.dispatchEvent(new Event("mouseover")));
    element?.addEventListener("mouseleave", () => {
      if (element.matches(":focus-within")) element.dispatchEvent(new Event("mouseover"));
    });
    element?.addEventListener("focusout", (event) => {
      if (
        event instanceof FocusEvent &&
        (!(event.relatedTarget instanceof Node) || !element.contains(event.relatedTarget)) &&
        !element.matches(":hover")
      ) {
        element.dispatchEvent(new Event("mouseleave"));
      }
    });
  }

  createEffect(
    () => undefined,
    () => {
      if (region && "showPopover" in region) region.showPopover();
    },
  );
  onCleanup(() => active?.hideToast());

  function Region() {
    return (
      <div
        {...stylex.attrs(styles.region)}
        id="site-notifications"
        ref={(element) => {
          region = element;
        }}
        popover="manual"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      />
    );
  }

  return { show, Region };
}

const slideIn = stylex.keyframes({
  from: { translate: "calc(100% + 32px) 0", opacity: 0 },
  to: { translate: "0 0", opacity: 1 },
});

const styles = stylex.create({
  region: {
    position: "fixed",
    inset: "auto",
    insetBlockStart: "max(20px, env(safe-area-inset-top))",
    insetInlineEnd: "max(20px, env(safe-area-inset-right))",
    margin: 0,
    padding: 0,
    borderWidth: 0,
    overflow: "visible",
    width: "max-content",
    maxWidth: "calc(100vw - 40px)",
    backgroundColor: "transparent",
    color: colors.notificationText,
    zIndex: 20, // Fixed-position fallback for browsers without Popover.
    pointerEvents: "none",
  },
  notice: {
    boxSizing: "border-box",
    display: "flex",
    alignItems: "center",
    gap: 16,
    paddingBlock: 8,
    paddingInlineStart: 20,
    paddingInlineEnd: 8,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.notificationBorder,
    borderRadius: radius.control,
    backgroundColor: colors.notificationSurface,
    color: colors.notificationText,
    boxShadow: `0 8px 28px ${colors.notificationShadow}`,
    fontFamily: type.body,
    fontSize: 17,
    lineHeight: 1.4,
    pointerEvents: "auto",
    animationName: { default: slideIn, [media.reducedMotion]: "none" },
    animationDuration: "320ms",
    animationTimingFunction: "cubic-bezier(0.16, 1, 0.3, 1)",
  },
  close: {
    width: 44,
    minWidth: 44,
    height: 44,
    borderWidth: 0,
    borderRadius: radius.control,
    backgroundColor: "transparent",
    color: colors.notificationText,
    fontSize: 16,
    cursor: "pointer",
    touchAction: "manipulation",
    outline: { default: null, ":focus-visible": `3px solid ${colors.focus}` },
    outlineOffset: { default: null, ":focus-visible": 2 },
  },
});
