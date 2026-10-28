# Rooftops Easter egg

Rooftops is a black-cat platformer with three levels. Closing the game window
resets the run and returns to the app.

![A black cat jumping toward cat-face coins above illuminated apartments](images/walkthrough/framed/rooftops-1.png)

## Controls

- **← →** move.
- **Space** jumps. Hold for a higher jump; release for a shorter hop.
- **Escape** closes the game window and resets the run.
- At the doorway, **Space** opens the next level. On the results screen, click
  **Play again** or press **Space** to start a new run.
- **Exit**, immediately beside **Play again**, closes the game window and resets
  the run while retaining your personal best.

## Playing

Jump across the rooftops to reach the lit doorway. Later levels have narrower
roofs, wider gaps, and more enemies.

The moon is the level indicator: a waxing crescent for level 1, a half moon for
level 2, and a waxing gibbous moon with a narrow dark crescent for level 3.
The small purple counters at the top left use a simple coin, a smooth symmetric
skull for deaths, and crossed swords for defeated enemies. Their shared flat
style keeps the counters visually consistent. Each count starts at zero and
carries across levels. The large floating clock at the top center counts total
active play time upward from 0:00; it pauses when the window loses focus and between levels.

Collect the cat-face coins by touching them. They are optional, and your total
carries between levels. Land on rats or owls to defeat them and bounce upward.
Touching an enemy from the side or falling returns you to the last checkpoint
roof, marked by a lit lantern. Collected coins remain yours.

After your first complete run, a translucent lavender cat replays your fastest
previous completion. It uses the same total active-play clock across all three
levels, including recorded deaths and checkpoint returns. Pausing the game or
waiting between levels pauses that clock for both cats. A ghost in another level
or outside the camera view is hidden: entering level 2 later than your best run
can mean its ghost is already farther ahead, or has moved on to level 3.
It cannot collect coins, defeat enemies, or collide with the player.

When the ghost is off screen, red **BEHIND** text on the right or green **AHEAD**
text on the left shows your gap in meters, with an arrow toward the ghost.
The gap updates continuously and disappears as soon as the ghost enters view.
Meters measure horizontal progress along the whole three-level course, so jumping
does not change the gap and comparisons still work across level boundaries.
The scale makes the cat's body about 0.4 m long and typical rooftop gaps about 2 m.

The opponent is fixed when a run begins. Setting a new fastest time replaces the
ghost for the next run; slower, tied, or unfinished runs keep the previous ghost.

Landing lights the buildings, revealing residents through their windows
vacuuming, playing piano, gardening, cooking, dancing, or stargazing.
Dark windows have closed blinds or shutters; lit windows leave the residents
fully visible. Three skyline layers scroll at different speeds
to create the parallax effect.

![A mid-level owl encounter above narrow rooftops](images/walkthrough/framed/rooftops-2.png)

## Results

Finishing the third level opens your results: total coins, kills,
deaths, and total completion time. Only a faster completed run replaces the
record; restarting or closing the game keeps it. Time spent paused or waiting between levels does not count.
**Download Scorecard** writes a 3600 × 2160 PNG to Downloads. Repeated saves use a new filename.
Both cards show a full moon and four centered columns for coins, deaths,
enemies defeated, and completion time. The footer shows `personal best: <time>`
when the current run is slower than the saved best, displaying that faster saved
time. First runs, ties, and new records have no personal-best footer. **Play again**, **Exit**,
and **Download Scorecard** share the same height, padding, and styling, with
widths that fit their labels.
The best time and its complete ghost replay are saved together in
`~/.gatto/rooftops.cbor`. Removing this file clears both. The game starts fresh
when no file exists; an unfinished run is never saved as a ghost.

After completing Rooftops, **App Preferences → General** includes **Reset Rooftops**
before **Reset app**.
Clicking it immediately clears the best time and ghost, restarts any open game
from level 1, and hides the button until you complete all three levels again.
The full **Reset app** action also clears Rooftops progress along with all other
local app data; see [reset and uninstall](../SUPPORT.md#reset-and-uninstall).

Moon geometry and icon references are recorded in
[the HUD research notes](research/rooftops-hud-2026-10-28.md).

![The results screen with coins, deaths, active play time, and the Download Scorecard button](images/walkthrough/framed/rooftops-3.png)

The saved scorecard:

![The downloadable Rooftops scorecard](images/walkthrough/rooftops-score.png)

## Screenshots

See the [contributor guide](../CONTRIBUTING.md#walkthrough-screenshots) to regenerate
the game, results, and scorecard images alongside the app walkthrough.
