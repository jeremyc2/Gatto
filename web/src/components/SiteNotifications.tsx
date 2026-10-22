import * as stylex from "@stylexjs/stylex";
import { createEffect, onCleanup } from "solid-js";
import { colors, media, radius, type } from "../design-system/tokens.stylex";

/** A single polite notice; repeated activations never build a stack. */
export function createSiteNotifications() {
  let region: HTMLDivElement | undefined;
  let active: HTMLDivElement | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let hovered = false;
  let focused = false;

  function pause() {
    clearTimeout(timer);
    timer = undefined;
  }

  function dismiss() {
    pause();
    active?.remove();
    active = undefined;
    hovered = focused = false;
    if (region && "popover" in HTMLElement.prototype && region.matches(":popover-open")) {
      region.hidePopover();
    }
  }

  function resume() {
    pause();
    if (active && !hovered && !focused) timer = setTimeout(dismiss, 6000);
  }

  function show(message: string) {
    if (!region || active) return;
    const trigger = document.activeElement;
    const notice = document.createElement("div");
    notice.className = stylex.attrs(styles.notice).class ?? "";
    const text = document.createElement("span");
    text.textContent = message;
    const close = document.createElement("button");
    close.type = "button";
    close.className = stylex.attrs(styles.close).class ?? "";
    close.textContent = "✖";
    close.setAttribute("aria-label", "Dismiss notification");
    close.setAttribute("popovertarget", "site-notifications");
    close.setAttribute("popovertargetaction", "hide");
    close.addEventListener("click", () => {
      dismiss();
      if (trigger instanceof HTMLElement && trigger.isConnected) {
        trigger.focus({ preventScroll: true });
      }
    });
    notice.addEventListener("mouseenter", () => {
      hovered = true;
      pause();
    });
    notice.addEventListener("mouseleave", () => {
      hovered = false;
      resume();
    });
    notice.addEventListener("focusin", () => {
      focused = true;
      pause();
    });
    notice.addEventListener("focusout", (event) => {
      focused = event.relatedTarget instanceof Node && notice.contains(event.relatedTarget);
      resume();
    });
    notice.append(text, close);
    active = notice;
    if ("popover" in HTMLElement.prototype && !region.matches(":popover-open")) {
      region.showPopover();
    }
    region.append(notice);
    resume();
  }

  createEffect(
    () => undefined,
    () => {
      if (region && "popover" in HTMLElement.prototype) region.showPopover();
    },
  );
  onCleanup(dismiss);

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
