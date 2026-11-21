/** Reflect a mounted notice in the top layer; fixed positioning is the fallback. */
export function setNotificationPopover(region: HTMLDivElement, open: boolean) {
  if (typeof region.showPopover !== "function") return;
  if (open && !region.matches(":popover-open")) region.showPopover();
  if (!open && region.matches(":popover-open")) region.hidePopover();
}

export function focusRemainsInside(event: FocusEvent & { currentTarget: HTMLDivElement }) {
  return event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget);
}

/** The caller's effect owns cancellation on pause, dismissal and unmount. */
export function expireNotification(dismiss: () => void) {
  const timer = setTimeout(dismiss, 6000);
  return () => clearTimeout(timer);
}
