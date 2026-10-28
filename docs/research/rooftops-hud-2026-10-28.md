# Rooftops moon and HUD references — 2026-10-28

## Moon phases

- [NASA: Moon phases](https://science.nasa.gov/moon/moon-phases/) shows the
  waxing sequence: crescent, first quarter (half illuminated), and gibbous
  (more than half illuminated). The terminator follows the projection of the
  spherical surface. This supports a curved inner boundary rather than a
  pie-chart sector or a straight crop for every phase.
- [Minecraft's MoonPhase enumeration](https://learn.microsoft.com/en-us/minecraft/creator/scriptapi/minecraft/server/moonphase?view=minecraft-bedrock-stable)
  distinguishes waxing crescent, first quarter, and waxing gibbous. Using a
  consistent waxing sequence gives the three levels distinct silhouettes.
- [Don't Starve Together's moon-cycle HUD](https://dontstarve.wiki.gg/wiki/Moon_Cycle/DST)
  uses a moon phase beside its clock. This is a useful example of communicating
  game state with a moon silhouette rather than numbered progress markers.

Rooftops uses 18%, 50%, and 86% illumination, with the lit limb consistently on
the right. A projected ellipse forms the terminator. Dim earthshine preserves
the round silhouette, craters are clipped to the illuminated face, and the halo
scales with illumination. The results card retains the final gibbous moon.

## Enemy counts

- [Team Fortress 2's official updates](https://www.teamfortress.com/?tab=updates)
  describe distinct weapon and enemy-action kill icons, including the Pomson,
  Wrap Assassin, and Zombie Demoman charge. Its compact pictograms communicate
  what caused a defeat without repeating text labels.
- [Fortnite's HUD controls](https://dev.epicgames.com/documentation/en-us/fortnite/deserted-domination-template-in-unreal-editor-for-fortnite)
  expose a dedicated elimination counter. Its [free-for-all guide](https://dev.epicgames.com/documentation/en-us/fortnite/create-a-free-for-all-game-in-fortnite-creative)
  also connects HUD score to elimination totals.

Design decision: reserve the skull for player deaths, since using it for both
deaths and kills would make unlabeled totals ambiguous. Use an original,
overlapping owl-and-rat silhouette for the single enemy count. The owl's ear
tufts and open eyes sit above the rat's round ear, pointed snout, and tail;
both fit inside the same 18 px slot as the coin and skull. The filled purple
shapes and generous negative space are intended to remain readable at a glance.
These are original drawings, not copied game assets.

The coin retains the existing kitten engraving, tinted with the app's
`#8b7bff` accent. Counters start at zero, accumulate across levels, and use the
same icon order on the final card. Elapsed time counts up because the measure
is the lowest time to finish all three levels; no countdown budget is defined.

## Refinement after visual review

The combined enemy silhouette was too easily mistaken for a cat, so the revised
HUD uses crossed swords for kills. Dense skull shapes are reduced optically
relative to the detailed coin and slender swords, rather than forcing identical
outline dimensions. The gameplay clock is 70 px, 3.5 times the initial 20 px size.

The final card celebrates completion with a full moon. Four equal-width columns
center both labels and values; the export measures actual font widths for the
same alignment. Personal best moves to the former Gatto footer as a subdued
`personal best: <time>` line. The Play again button is clickable, and Space
continues to restart the game without a visible shortcut prompt.

## Icon consistency pass

The detailed kitten engraving made the purple stat coin clash with the flat
HUD symbols. Replace that stat icon with a simple circular coin rim and an
inset denomination mark. The skull now mirrors one continuous curved half,
with equal eye sockets and inset teeth so its outer contour stays smooth.
The swords share one blade construction with rounded guards and pommels,
rotated as a symmetric pair. All stat artwork is shared vector geometry in
the gameplay HUD, results, and downloaded PNG. The action label is now
Download Scorecard.

## Personal-best footer and paired actions

Capture the saved record when a run begins. Only a strictly faster completion
shows the footer with its new best time; omit it for first runs, ties, and slower
completions. Both native results and PNG export use that same comparison.
Play again and Download Scorecard share one style with equal dimensions, padding,
corner radius, typography, and interaction colors.

Button widths were subsequently changed to fit their labels; shared height,
padding, and styling do not require identical widths.

## Corrected footer visibility

The intended comparison is to show the saved best on slower runs, so the player
can see the faster time to aim for. The footer displays the saved record, not the
current completion time. First runs, ties, and new records omit this footer.
This supersedes the earlier reversed comparison.

## Ghost playback and storage

Ghosts sample the player's actual poses up to 30 times per active second, with
explicit samples for level changes, deaths, and completion. Interpolation uses
total elapsed play time rather than restarting at each level. Level changes and
checkpoint respawns never interpolate across different locations. The current
opponent remains fixed until the next run begins.

The best completion and replay share a versioned `rooftops.cbor` file. The local
development-only JSON record was removed at the user's request; there is no
migration path to maintain. [CBOR](https://cbor.io/) supports the repeated numeric
pose data in a compact binary format. [Ciborium](https://docs.rs/ciborium/0.2.2/ciborium/)
integrates it with Serde and encodes numbers at the smallest lossless size.
Saves run in the background, merge only faster results, and atomically replace
the file. Unsupported or invalid recordings are not used for playback.

## Off-screen ghost distances

Use 64 world units per meter: the cat's 27-unit body represents about 0.42 m,
and typical gaps of 105–141 units represent about 1.6–2.2 m. The comparison
uses horizontal course progress, rather than screen pixels or jump height.
Each level contributes the distance from its starting position to its doorway;
the next level starts at the same cumulative distance. This makes the gap
continuous across transitions and gives the correct direction even when the
ghost's local coordinates are in another level.

A ghost ahead of the player gives a red Behind indicator at the right edge;
a ghost behind gives a green Ahead indicator at the left edge. A directional
arrow and one decimal place make the gap readable as it changes. Suppress the
indicator as soon as any part of the ghost is horizontally visible in the same
level. Offsets and meter scale are independent of camera movement and zoom.

## Reset Rooftops

Preferences reads only completion metadata from CBOR on startup, and listens
for game-completion events thereafter. Reset Rooftops appears in General only
after a completion. It acts immediately, clears the stored best and replay,
restarts an open game, and hides until another completion. It has no confirmation
dialog. Saves capture a per-file generation; reset increments it under the same
lock used for writes, so an older queued save cannot recreate the cleared data.

Reset Rooftops precedes Reset app in the General actions. The full app reset
invalidates pending Rooftops saves and clears any open game before removing the
entire `.gatto` directory, including unrecognized local data. Normal startup
then creates fresh state.
