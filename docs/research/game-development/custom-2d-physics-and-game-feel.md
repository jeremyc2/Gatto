# Building custom 2D indie physics that feels good

Research date: October 18, 2026 (America/Chicago).

Scope: small, primarily single-player 2D games made by a solo developer or small
team. This is research and a proposed development method, not an implementation
plan for an existing Gatto game. Platformers supply useful concrete examples;
the method also applies to bouncing, drifting, launching, and pushing games.

**Recommendation:** build the smallest physical model that supports an enjoyable
repeatable action, then develop its controls, challenges, and feedback together.
For a precision platformer this might mean rectangles, gravity, and explicit
movement rules. For a momentum game it might mean circles, impulses, drag, and
bounces. A general-purpose physics engine is a separate, much larger undertaking.

The cited case studies below are primary evidence: developer accounts, released
source, original research, and engine documentation. The experiment protocols,
example parameters, acceptance criteria, and effort guidance are this note's
synthesis. They are proposals to validate, not universal recipes or measured
proof of fun. Historical articles are identified by their original dates where
available; recent crawl dates are not publication dates.

## 1. Decide what kind of physics you actually need

| Approach | Good initial fit | Work you own | Main limitation |
| --- | --- | --- | --- |
| Custom movement and simple collision | Precise platforming, top-down movement, a ball in a small arena | Movement, shape tests, contact response, game rules | Slopes, moving platforms, and fast objects need deliberate treatment |
| Custom controls over engine collision queries | Games needing authored movement with more varied geometry | Movement and response policy; engine finds contacts | Understand the engine's collision semantics and update order |
| Rigid-body engine plus custom game rules | Stacks, rotating objects, joints, mechanisms, physical puzzles | Game design, controller, materials, constraints, tuning | Realistic interaction does not automatically produce satisfying control |
| General physics engine written from scratch | Learning physics-engine construction is itself a goal | Collision, solver, stability, contacts, constraints, optimization | Substantial work beyond proving a small game's core action |

These are scope recommendations. The middle options still let you design your
own physics behavior. Godot's character body explicitly leaves gravity and
friction to user code while providing movement collision. Box2D also exposes
collision utilities independently of rigid-body simulation.
[Godot character bodies](https://docs.godotengine.org/en/stable/tutorials/physics/using_character_body_2d.html),
[Box2D collision interface](https://box2d.org/documentation/md_collision.html).

Start by listing the interactions the first playable actually requires. If it
needs only a character jumping between stationary rectangles, defer rotating
crates, rope simulation, water, slopes, and pushable stacks. If balancing stacks
is the actual idea, bring a rigid-body engine into the first prototype; removing
that behavior would remove the experiment's purpose.

The expensive part is often combinations: a moving platform pushing a player
into a wall while a jump is buffered, or two rotating objects pinning a third.
Every added interaction expands the situations that need defined behavior.

## 2. What makes the physics fun?

The MDA framework separates implemented mechanics, the behavior that emerges
while playing, and the player's emotional experience. Its practical contribution
here is to reason in both directions: begin with the feeling you want, propose
rules that might produce it, then inspect what actually happens in play.
[MDA, Hunicke, LeBlanc, and Zubek, 2004](https://www.cs.northwestern.edu/~hunicke/MDA.pdf).

Write a short experience target before touching coefficients. For example:

> The player should feel agile, able to recover from small mistakes, and proud
> when they link several movements without stopping.

Or:

> The player should feel like they are steering a heavy puck: anticipate the
> turn, commit, then use a satisfying ricochet to recover speed.

Those targets imply different acceleration, braking, air control, and forgiveness.
My proposed criteria are:

| Quality | What to look for in play |
| --- | --- |
| Agency | The player can connect their input to an outcome and intentionally change it |
| Predictability | Similar situations behave similarly enough to support learning |
| Readability | Motion, collision shapes, sound, and camera explain what happened |
| Expressiveness | Timing, direction, or commitment allow meaningfully different outcomes |
| Mastery | Practice produces visible improvement or new useful techniques |
| Recovery and stakes | Mistakes create a fitting consequence and a reason to try again |

A useful mechanical seed is one action with more than one purpose. Downwell's
publisher describes gunboots that both attack and slow the player's descent;
Fumoto's GDC session describes developing the game around a central discovered
mechanic. My inference is to look for similarly productive overlaps: a dash that
also commits you to danger, or a bounce that is simultaneously traversal and
scoring opportunity.
[Downwell, publisher description](https://www.devolverdigital.com/games/downwell),
[Fumoto, GDC 2016 session overview](https://www.gdcvault.com/play/1023533/Polishing-the-Boots-Designing-Downwell).

A pleasant movement toy is a promising start. Add a goal that makes its properties
matter: a destination, a target, a combo to preserve, a structure to stabilize,
or a route with different risks. In their first-person account, the Experimental
Gameplay Project team describes short prototypes, building the toy first, and
finding goals inherent in the interaction. Their one-week constraint was their
experiment, not a delivery estimate for a finished game.
[Experimental Gameplay Project, 2005](https://www.gamedeveloper.com/game-platforms/how-to-prototype-a-game-in-under-7-days).

## 3. Lessons from developers who built these systems

### Celeste and TowerFall: a constrained collision core

Maddy Thorson describes Actors and Solids with integer-positioned, axis-aligned
rectangular colliders. Fractional motion accumulates; Actors check movement one
pixel at a time on each axis. Individual Actor classes own their velocity,
gravity, and collision response. Moving platforms require explicit carrying,
pushing, and squishing policies. Solids do not collide with other Solids.
[Celeste & TowerFall Physics](https://www.maddymakesgames.com/articles/celeste_and_towerfall_physics/index.html).

The useful lesson is a narrow model with deliberate rules. The article describes
years of refinement; it does not establish that a finished platformer controller
is a tiny weekend task. The accompanying player-code commentary also explains
why tightly ordered, sequential behavior suited that particular game and team.
[Official player-code commentary](https://github.com/NoelFB/Celeste/blob/master/Source/Player/Readme.md).

### Celeste: interpret intended actions generously

Thorson documents coyote time, jump buffering, reduced gravity near a held jump's
peak, corner corrections, and inherited moving-platform momentum. These widen
timing and positioning allowances in the player's favor. The article does not
supply universal buffer or coyote durations; its pixel allowances belong to
Celeste's resolution and scale.
[Celeste & Forgiveness](https://www.maddymakesgames.com/articles/celeste_and_forgiveness/index.html).

The design inference: reserve difficulty for the decisions and execution the
game is meant to test. Investigate an apparently ignored, narrowly mistimed jump
as a possible control problem before making the obstacle easier.

### Action Henk: preserve interesting momentum

RageSquid's Roel Ezendam describes jumping through added force, with direction
influenced by the ground angle. Existing upward motion could produce excessive
height. The team reduced the effect rather than eliminating it, retaining
momentum-sensitive decisions that support improving racing times.
[Ezendam's developer account, 2015](https://www.gamedeveloper.com/design/game-design-deep-dive-i-action-henk-s-i-jump-physics).

The inference: maximum braking and immediate reversal are not universal goals.
Some games derive their identity from commitment and inertia. Decide whether an
unexpected technique is useful depth, an unclear rule, or an exploit before
removing it.

### Ultimate Chicken Horse: a mechanic includes communication and combinations

Clever Endeavour describes exploring a diagonal boost pad and an automatic
cannon. The boost pad's effect worked but was difficult to communicate visually.
The cannon was compelling, then required months of trajectory, collider, art,
animation, QA, and beta-test work across players, projectiles, and networking.
[Clever Endeavour, 2020](https://richard-atlas.squarespace.com/blog/2020/9/23/behind-the-scenes-the-cannon).

This is one feature's history, not a general schedule. It illustrates why a
successful first interaction and a reliably finished mechanic are different
milestones.

## 4. Build a stable foundation for experimentation

The following is a proposed architecture for a small game, not a reconstruction
of any cited game's implementation:

1. **Input:** capture held directions and individual press/release events.
2. **Movement rules:** determine desired velocity, gravity mode, jump eligibility,
   dash state, or applied impulse.
3. **Collision:** attempt motion and return the actual displacement and contacts.
4. **Gameplay:** interpret contacts as landing, bounce, damage, pickup, or failure.
5. **Presentation:** animate, play sound, and move the camera from those events.

Give each body's motion one authoritative owner. Avoid having animation,
collision response, and a controller independently rewrite its position.
Define precedence where rules compete: death versus buffered jump, landing
versus dash refresh, or platform carry versus knockback.

### Time, input, and rendering

Use a fixed simulation tick; 60 Hz is a reasonable initial experiment. Accumulate
elapsed real time and run whole ticks independently of rendering. Leave CPU
headroom and bound catch-up work after a stall. Define whether excess elapsed
time is discarded or the game slows; neither policy recovers missing simulation
for free. Interpolating previous/current simulation states can smooth rendering
but introduces roughly one tick of presentation delay. A fixed tick helps
repeatable debugging; it alone does not guarantee cross-platform determinism.
[Fiedler, Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/).

Box2D likewise recommends fixed stepping and distinguishes the main simulation
step from internal substeps used for accuracy. Its suggested substep count is
specific to its solver; it is not a requirement to run a custom rectangle
controller four times per tick.
[Box2D simulation documentation](https://box2d.org/documentation/md_simulation.html).

Implementation recommendations:

- Preserve presses and releases that occur between ticks. Consume a press once,
  even if several simulation ticks run before the next rendered frame.
- Keep held state separate from a queued press: an early press followed by an
  early release should have an explicitly chosen short-hop/buffer behavior.
- Use seconds or simulation ticks consistently for grace windows. Specify which
  boundary tick counts; test the tick immediately before and after expiry.
- Keep interpolated render positions out of collision calculations. Reset render
  history after a teleport or respawn.
- Measure on the intended input device and display. A correct fixed-step
  controller can still feel delayed because of input sampling or presentation.

### Collision rules before content

For an initial rectangular platformer, separate-axis motion with obstacle checks
is a tractable option. Preserve fractional movement if snapping collision
positions to a grid. Define floor, wall, and ceiling contacts separately; cancel
only the velocity component blocked by the surface.

For a bouncing circle, use the contact normal to decide the response. Against a
stationary wall, with unit normal `n` pointing toward the circle, and only when
`dot(v, n) < 0`, a simple reflection is:

```text
v_after = v_before - (1 + restitution) * dot(v_before, n) * n
```

Here restitution 0 removes the incoming normal velocity, while 1 preserves its
magnitude. This formula does not solve rotating bodies or collisions between two
moving masses; those require more work.

Metanet's original N tutorial distinguishes collision detection from response,
explains projection and velocity response, and reports preferring its simple
response over a more realistic alternative. It also explains why testing only
final overlaps fails for some fast/small objects.
[Metanet, N Tutorial A](https://www.metanetsoftware.com/technique/tutorialA.html).

Choose a deliberate fast-motion policy: inspect sufficiently small motion
increments, or sweep the moving shape to find its first contact. A ray alone
does not represent the width of a player. After a bounce, handle remaining tick
time with bounded repeated collision processing. Separately define recovery
from initial overlap. Box2D's documentation describes shape casts and time of
impact, including limitations.
[Box2D collision queries](https://box2d.org/documentation/md_collision.html).

If required, write explicit policies for one-way platforms, dropping through,
slopes, riding moving geometry, inherited launch velocity, and crushing. These
are game rules, not details to leave to incidental overlap order.

## 5. Dial in outcomes, then derive coefficients

Expose parameters in terms you can judge while playing:

| Design question | Useful parameter |
| --- | --- |
| How quickly do I get moving? | Time to reach target speed |
| How far do I slide after release? | Stopping distance or stopping time |
| How high and how long is a jump? | Height and time to apex |
| How quickly does the landing arrive? | Fall duration or fall-gravity multiplier |
| How far does a dash carry me? | Dash distance and duration |
| How long does a puck keep moving? | Velocity half-life |
| How much does a bounce retain? | Normal restitution and tangential loss |

Mark Brown's Platformer Toolkit implementation independently exposes running,
stopping, turning, ground/air behavior, jump height, apex time, and fall behavior.
It is a concrete example of authoring movement through understandable controls.
[Behind the Code](https://gmtk.itch.io/platformer-toolkit/devlog/395523/behind-the-code).

### Worked example: a jump baseline

For constant downward acceleration, no drag, upward-positive coordinates, jump
height `h`, and apex time `T`:

```text
gravity_magnitude = 2*h / T²
initial_upward_speed = 2*h / T
```

With downward-positive screen coordinates, negate the initial upward velocity
and add positive gravity. Kyle Pittman's talk derives designer-facing jump
parameters and discusses piecewise arcs and integration choices.
[Building a Better Jump, slides 15–19 and 27–39](https://media.gdcvault.com/gdc2016/Presentations/Pittman_Kyle_BuildingBetterJump.pdf).

These are deliberately invented prototype targets, not recommended industry
constants or Celeste's numbers:

| Target | Example | Derived baseline |
| --- | --- | --- |
| Full jump | 64 world-pixels high, apex at 0.30 s | Gravity ≈ 1,422.22 px/s²; launch ≈ 426.67 px/s upward |
| Faster descent | Fall 64 px from rest in 0.24 s | Fall gravity ≈ 2,222.22 px/s² |
| Running | 240 px/s reached from rest in 0.12 s | Constant acceleration 2,000 px/s² |
| Braking | Stop from 240 px/s in 0.08 s | Constant braking 3,000 px/s²; stopping distance 9.6 px |
| Dash | 96 px in 0.12 s | 800 px/s if speed is constant |

A full uncut jump in this idealized two-gravity example lasts 0.54 s. At an
already-established constant horizontal speed of 240 px/s, its reference-point
travel is 129.6 px. This is not the guaranteed clearable edge-to-edge gap:
collider width, initial placement, acceleration, target elevation, and landing
policy also matter.

Changing apex time while holding height fixed requires recomputing both launch
speed and gravity. Otherwise you unintentionally change several outcomes.
Changing gravity near the apex, cutting the jump on release, or applying a
terminal speed also changes the baseline trajectory; remeasure after each.

Discrete integration matters. For this constant-acceleration segment, advancing
position by `v*dt + 0.5*a*dt²` before advancing velocity gives the analytic
free-flight position at tick boundaries. A velocity-first Euler update has a
step-size-dependent height error. Neither eliminates collision timing errors;
split motion at contacts or state changes if their timing matters. Measure the
actual controller's height, duration, and travel rather than trusting its config.
[Pittman, integration slides](https://media.gdcvault.com/gdc2016/Presentations/Pittman_Kyle_BuildingBetterJump.pdf).

### An equally small non-platformer baseline

For an unforced puck whose velocity should halve every `H` seconds:

```text
velocity *= 2 ** (-dt / H)
```

At `H = 0.25 s`, after 0.50 s it retains 25% of its starting speed. This is an
exponential drag design, not constant-distance braking. Add input thrust and
collision response separately, then measure the combined system.

The point of both examples is to choose observable behavior before guessing
coefficients. The equations establish a baseline; play determines whether that
baseline is desirable.

## 6. A repeatable tuning loop

This is a proposed practical protocol, not a claim that the cited developers all
used this exact process.

### Step 1: make one action playable

Build an empty room, one controllable shape, and one verb: jump, launch, drift,
or bounce. Add immediate restart. Include enough sound and animation to judge
the intended sensation, while keeping them individually switchable for diagnosis.
Avoid beginning with a whole level, progression system, or general engine.

### Step 2: build a permanent test room

Include a flat acceleration lane, a stopping marker, turns, and representative
contacts. For platforming, add several gap widths, low ceilings, corners, narrow
landings, and repeated jumps. For a puck game, add glancing and head-on walls,
small targets, narrow passages, and a recovery route after a missed shot.

Keep this geometry stable during comparisons. Separately test changed level
geometry so a movement improvement is not confused with an easier course.

### Step 3: make the state visible

Add pause, single-tick advance, slow motion, spawn/reset markers, trails,
collision outlines, and a compact readout of velocity and movement state. Log
press times, accepted/rejected actions with reasons, contact events, apex height,
landing position, and elapsed simulated time.

Save named parameter presets. Record the build, level revision, tick rate,
initial state, input sequence, and random seed if relevant. Replays are valuable
within a controlled build; bit-identical replay across machines requires more
than a fixed timestep.

### Step 4: explore broadly, then narrow

Make two or three deliberately distinct presets, such as immediate control,
moderate momentum, and strong momentum. First choose which supports the intended
experience. Then bracket a parameter: compare clearly too little against clearly
too much, reduce the interval, and retest. Tiny random changes are hard to judge.

Tune in dependency order: scale and camera framing; speed/acceleration/braking;
core flight or bounce; control during movement; forgiveness; special moves;
feedback intensity. Revisit earlier choices when a later rule changes their
meaning. Change a small coherent parameter group, preserving derived constraints
such as fixed jump height.

### Step 5: diagnose a specific complaint

| Observation | Investigate before changing |
| --- | --- |
| “It ignored my jump” | Input capture, buffer expiry, grounded state, eligibility, tick order |
| “It's floaty” | Total airtime, apex duration, fall speed, air control, camera lag |
| “It's slippery” | Stop distance, reversal delay, input dead zone, surface rules |
| “I hit that” | Collider/hurtbox, displayed pose, contact timing, effects origin |
| “I can't see the landing” | Camera framing, look-ahead, scenery, visual effects |
| “It's easy but boring” | Repeated decisions, risk/reward, route variety, mastery opportunities |

These are hypotheses, not automatic fixes. Reducing airtime will not repair a
jump press that was lost between ticks.

### Step 6: compare on a controlled task

Example experiment: players repeatedly overshoot a landing. Keep maximum speed,
course, jump height, and camera fixed. Compare braking to zero in 0.08 versus
0.12 seconds, then inspect overshoot and player preference. Under the constant
braking example above, those choices imply 9.6 versus 14.4 px stopping distance.
Recheck reversal and any momentum-based challenge before keeping the change.

Log each experiment in a compact form:

```text
Observation: New players overshoot the first narrow landing.
Hypothesis: Braking distance is longer than the space suggests.
Change: Preset B reduces braking time; speed and jump are held fixed.
Task: Same course and starting state; alternate preset order.
Evidence: Note misses, correction inputs, explanations, and preference.
Decision: Keep/revert; identify which other scenarios need rechecking.
```

Do not invent an improvement percentage before collecting results. Small
playtests provide direction and failure examples, not a population-wide verdict.

### Step 7: put the action under meaningful pressure

Make a tiny complete challenge with a goal and a consequence: a short route,
a target sequence, or a score/combo opportunity. Combine the verb with one other
rule. Compare a safe path and a tempting risky path. Ask whether the interaction
still offers decisions after the novelty wears off.

If it works only in the empty room, investigate the challenge design. If it
requires a new mechanic every few seconds to remain interesting, investigate
whether the core action has enough range.

## 7. Playtest control and enjoyment separately

Use fresh players who resemble the intended audience, alongside experienced
players who can expose deeper techniques and exploits. Observe before coaching.
Give each a task, then ask what they expected, why a miss happened, and what they
would try differently. Let them describe the sensation before suggesting names
like “floaty.” Alternate preset order so familiarity is less likely to decide
the comparison. Repeated testing with the same person measures learning too.

Useful observations include time to the first intentional success, repeated
unexplained misses, corrective inputs, recovery after errors, route variety,
and whether players voluntarily retry. Ask about enjoyment and frustration
directly. No single count proves fun: retries can indicate pleasure or confusion,
and lower failure can mean better controls or an uninteresting challenge.

For forgiveness, first inspect actual near-miss timings. At 60 Hz, experiments
with 4, 6, or 8 ticks mean about 66.7, 100, or 133.3 ms. These are candidate test
points, not prescribed windows. Keep the smallest allowance that serves the
experience without making timing feel detached. Consume jump eligibility and
queued input so one press cannot create repeated jumps. Test release-before-
landing and simultaneous conflicting actions explicitly.

Evaluate presentation both enabled and reduced. Jonasson and Purho's
Juice It or Lose It demonstrates adding feedback to a simple game; their
published Juicy Breakout source is a useful study of that layer.
[GDC session](https://www.gdcvault.com/play/1016487/Juice-It-or-Lose),
[original example source](https://github.com/grapefrukt/juicy-breakout).

My recommendation is to make feedback explain the event: stronger impacts sound
and look stronger, landing shows contact, and a successful sequence has a clear
rhythm. Keep collision geometry stable while the sprite squashes. Tune camera
follow separately from movement; offer reduced shake if shake is part of the
treatment. Ryan Juckett's original damped-spring article provides a mathematical
basis for smooth camera tracking with time-consistent behavior.
[Juckett, Damped Springs](https://www.ryanjuckett.com/damped-springs/).

## 8. When is the physics sufficiently locked down?

Treat “locked” as a versioned movement contract that level design can rely on.
Proposed exit criteria:

- Required movement is reliable across representative geometry and supported
  input devices; there are no known blocking collision failures in that scope.
- The test room reproduces target heights, times, distances, and allowed
  boundary cases within explicitly chosen tolerances.
- The same tick-indexed input sequence gives the expected simulation result
  under different render rates in the controlled build; stalls and resume work
  according to the chosen time policy.
- Fresh players understand basic control and can explain most outcomes.
- Experienced players find useful improvement opportunities, with accepted
  emergent techniques recorded as rules.
- A small complete challenge remains enjoyable across repeat attempts.
- Presets, level revision, contact policies, and regression scenarios are saved.

Use small automated checks for collision math, event consumption, buffer expiry,
and trajectory targets. Use replays and human play for feel. Avoid freezing
incidental per-frame positions when what matters is an allowed outcome range.
A discovered technique needs a deliberate keep/change decision; preserving
mastery does not mean preserving every bug.

After this gate, bias toward tuning obstacles, enemies, rewards, and level
spacing around the movement. Reopen core physics when evidence warrants it,
then recheck dependent levels. Otherwise each coefficient change silently moves
the foundations under existing content.

## 9. What it takes in practice

You need basic vectors and kinematics, simple shape collision, explicit state
and event handling, a stable update loop, and tools that shorten the interval
between changing a value and judging it. More advanced geometry and solver
knowledge becomes necessary when the game's interactions demand it.

The first milestone can be very small: one room, one shape, one interesting
verb, named presets, visible collision, and a short challenge. The refinement
work is open-ended until you define the audience, target sensation, input
devices, geometry, and interactions. A working jump is not a reliable estimate
of the cost of a finished controller. Multiplayer adds another scope boundary
through synchronization, latency, and replay requirements; evaluate it early
if it is essential.

A sensible sequence of experiments is: prove the toy, stabilize collision, derive
a movement baseline, test contrasting presets, add a meaningful challenge,
observe fresh players, then capture the movement contract. Repeat failed stages
without expanding the feature list merely to create progress.

The most useful question during tuning is: **what decision or sensation should
this change improve, and what observation would tell us it did?**

## Suggested source trail

These are the most useful next reads; the full note links each factual claim
near its source.

1. [Platformer Toolkit](https://gmtk.itch.io/platformer-toolkit): a playable way to experience parameter changes; pair with its linked code explanation.
2. [Celeste & TowerFall Physics](https://www.maddymakesgames.com/articles/celeste_and_towerfall_physics/index.html): a deliberately constrained custom collision model.
3. [Celeste & Forgiveness](https://www.maddymakesgames.com/articles/celeste_and_forgiveness/index.html): concrete timing and spatial accommodations.
4. [Building a Better Jump](https://media.gdcvault.com/gdc2016/Presentations/Pittman_Kyle_BuildingBetterJump.pdf): authoring trajectories from outcomes and understanding integration.
5. [Action Henk's jump physics](https://www.gamedeveloper.com/design/game-design-deep-dive-i-action-henk-s-i-jump-physics): momentum, slope-dependent choices, and intentional partial correction.
6. [N Tutorial A](https://www.metanetsoftware.com/technique/tutorialA.html): detection, response, and fast-motion problems from the developers of N. The explanations remain useful; its legacy Flash demonstrations may not run.
7. [Ultimate Chicken Horse cannon development](https://richard-atlas.squarespace.com/blog/2020/9/23/behind-the-scenes-the-cannon): the distance between a promising mechanic and a finished one.
