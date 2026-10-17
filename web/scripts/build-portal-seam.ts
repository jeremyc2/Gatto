// Derive both sides from one contour; never hand-edit the complementary asset.
const source = await Bun.file(new URL("../assets/images/portal-seam.svg", import.meta.url)).text();
const contour = source.match(/<path[^>]*\sd="([^"]+)"/)?.[1];
if (!contour?.startsWith("M0 0V") || !contour.endsWith("V0Z")) {
  throw new Error("Expected the 768 × 48 tear to close along its top edge.");
}
// Keep every cut coordinate; close the other half along the bottom instead.
const lowerContour = contour.replace(/^M0 0V/, "M0 48V").replace(/V0Z$/, "V48Z");
const complement = source.replace(/<path[^>]*\/>/, `<path fill="#fff" d="${lowerContour}"/>`);
await Bun.write(
  new URL("../assets/images/portal-seam-complement.svg", import.meta.url),
  complement,
);
