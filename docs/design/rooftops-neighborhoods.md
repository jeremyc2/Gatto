# Rooftops neighborhoods

The three levels take a fictional nighttime route through distinct New York
building traditions. Their materials, cornices and window coverings establish
the neighborhood while the large illuminated rooms remain the visual focus.

| Level | Facades | Window details |
| ----- | ------- | -------------- |
| 1 — Brownstone blocks | Warm brownstone and red brick, staggered joints, stone courses and projecting bracketed cornices. Neighboring buildings vary subtly in hue and block size. | Painted green louvred shutters, stone lintels with small keystones and broad sills. Open shutters fold into the narrow spaces beside the rooms. |
| 2 — Loft district | Green-gray cast-iron framing, fine masonry infill, narrow columns and repeated bolted floor beams. | Ribbed metal shutters, thin iron surrounds and small column capitals. The open bays keep the same clear view of the residents. |
| 3 — Stone towers | Pale warm-gray dressed blocks, stepped cornices, fluted corner piers and small geometric spandrel details. | Recessed horizontal blinds, dark bronze side edges and layered stone lintels. |

Palettes and coverings are authored for the game's night lighting. The room
opening stays 66 × 62 world units, with the same spacing, warm interior colors,
resident poses and landing-triggered opening. Exterior trim occupies the narrow
spaces beside and between openings; nothing is added over an illuminated room.

[The facade renderer](../../src/rooftops/neighborhood.rs) owns these materials
and surrounds. It paints masonry and roof trim together in one native element
per visible building. Joints are anchored to each building so scrolling cannot
make its texture move independently. Cornice ornament extends below the existing
roof surface; it does not add platforms or alter jump geometry.

At a level handoff, the shared starting building retains its previous level's
neighborhood, material variation, windows and residents. Incoming buildings use
the next neighborhood. Both sides are visible during the slide, making the
change of area part of the existing continuity sequence. Each small rooftop
shelter uses the supporting building's wall, coping and frame palette; its open
doorway has no handle. The shared shelter retains that palette into the next
level. Static facade, shutter and surround geometry is cached per game window
and discarded on resize; camera movement changes only its placement.

Gameplay heights, widths, rooftop gaps, enemy kinds and enemy timing are
independent of this appearance layer. Controls and capture commands live in
[the game guide](../EASTER_EGG.md) and [contributor guide](../../CONTRIBUTING.md#walkthrough-screenshots).

## Architectural references

- The NYC Landmarks Preservation Commission's [Linden Street Historic District
  announcement](https://www.nyc.gov/site/lpc/about/pr2023/lpc-designates-the-linden-street-historic-district-in-bushwick-brooklyn.page)
  describes brick and brownstone row houses with continuous decorative cornices.
  These inform the first level's warm masonry and roofline details.
- Its [361 Broadway designation report](https://s-media.nyc.gov/agencies/lpc/lp/1225.pdf)
  describes repeated cast-iron bays, columns and concentrated ornamental details.
  These inform the second level's repeated metal framing.
- Its [East Midtown landmark announcement](https://www.nyc.gov/site/lpc/about/pr2016/11-22.page)
  describes limestone, brick, terra cotta, continuous piers and ornamental setbacks
  on the Hotel Lexington. These inform the final level's stone palette and
  restrained geometric trim.
