const arrivalKey = "gatto.rooftops.portal-arrival";
const maxAge = 12_000;

export function savePortalArrival(time: number): void {
  try {
    sessionStorage.setItem(arrivalKey, JSON.stringify({ time, expires: Date.now() + maxAge }));
  } catch {
    // The game still opens if session storage is unavailable.
  }
}

export function takePortalArrival(): number | undefined {
  try {
    const saved = sessionStorage.getItem(arrivalKey);
    if (saved === null) return;
    sessionStorage.removeItem(arrivalKey);
    const arrival: unknown = JSON.parse(saved);
    if (
      arrival &&
      typeof arrival === "object" &&
      "time" in arrival &&
      typeof arrival.time === "number" &&
      Number.isFinite(arrival.time) &&
      arrival.time >= 0 &&
      "expires" in arrival &&
      typeof arrival.expires === "number" &&
      arrival.expires >= Date.now()
    ) {
      return arrival.time;
    }
  } catch {
    // A direct visit starts at the ordinary menu frame.
  }
}
