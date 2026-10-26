# Keeping a game small enough to finish

Research date and source access date: October 18, 2026 (America/Chicago).

**Recommendation:** preserve one compelling player experience, prove it in a
playable form, measure the cost of producing it, and define a complete first
release before expanding. Treat additional ambition as a choice that must earn
its cost, rather than as unfinished obligations inherited from the original idea.

This note addresses development scope and complexity. No particular game concept,
genre, budget, or team has been specified. The practical workflow and decision
rules below are a synthesis to adapt, not a researched universal schedule. The
evidence consists of firsthand developer writing, an official conference session
description, and platform documentation. It does not establish that any method
guarantees commercial success.

## 1. Separate the kinds of complexity

Use these working distinctions when reviewing a design:

| Question                              | What to examine                                                                 |
| ------------------------------------- | ------------------------------------------------------------------------------- |
| What must a player learn?             | Rules, controls, vocabulary, UI, exceptions, and information presented together |
| What must a developer maintain?       | State, dependencies, tools, interactions, persistence, and special cases        |
| How much must be produced?            | Locations, encounters, characters, writing, art, audio, and their variations    |
| What makes continued play worthwhile? | Interesting choices, mastery, discovery, emotional development, or expression   |

These are related, but should not be collapsed into one feature count. David
Mullich distinguishes learning/rule complexity from depth through interesting
decisions; he also treats engagement as broader than those two properties.
That is a useful practitioner framework, rather than an objective scale for
ranking every genre. [Mullich, November 21, 2016](https://davidmullich.com/2016/11/21/designing-a-good-game-is-about-more-than-making-it-difficult-complexity-depth-and-balance/).

**Proposed review:** for every new system, name the particular experience or
decision it adds. Then name the new concepts the player must understand and the
new cases the team must support. Prefer additions whose benefit is visible in
play. A simpler presentation can help players without reducing production cost;
a hidden simulation still needs implementation and testing.

Avoid assuming that a sophisticated simulation will become an enjoyable game
once enough supporting features exist. Daniel Cook reports that his _Secret Life
of Aliens_ prototype had an original gossip simulation but opaque cause and
effect and slow feedback. His redesign simplified the model into explicit
tokens, rules, and player actions. This is a firsthand failure and redesign,
not proof that all simulation should be simplified or that this redesign shipped
successfully. [Cook, August 10, 2005](https://lostgarden.com/2005/08/10/the-secret-life-of-aliens-a-redesign/).

## 2. Write the promise before the feature list

Create a short scope agreement with these fields:

- **Player promise:** what the player does and why it should be worth doing.
- **Essential experience:** the interaction, atmosphere, relationship, or story
  effect that must survive every cut.
- **First release:** the smallest complete experience you would honestly offer
  to a player, including how a session begins and reaches a satisfying stopping
  point.
- **Capacity:** available working time, money, existing skills, and help actually
  committed to the project.
- **Exclusions:** appealing directions that this release deliberately leaves for
  later consideration.
- **Next uncertainty:** the question a playable experiment must answer before
  more production is justified.

Do not make the promise so generic that every cut technically preserves it.
“Explore a world” cannot distinguish a compelling compact game from an empty
map. A better promise identifies the distinctive experience: for example,
“find surprising routes by gliding and climbing around a place that feels
welcoming.” This is an illustrative description, not a prescribed genre.

Likewise, do not cut the defining ambition reflexively. If the promise is friends
coordinating online, the early experiment needs that coordination and enough
network behavior to test it. A solo substitute may answer a different question.
Shrink the number of spaces or objectives around the essential experience first.
If the defining requirement itself exceeds capacity, explicitly pause or change
the concept rather than pretending an unrelated prototype validates it.

The official GDC 2020 description of Adam Robinson-Yu's _A Short Hike_
postmortem records that he paused a larger project and made a small open-world
game whose initial release fit a four-month deadline. This supports the
possibility of preserving an appealing experience at a smaller scale. Four
months is that project's history, not an estimate for another developer. Only
the session description was reviewed here; detailed claims about the talk's
specific techniques are excluded. [GDC session description, 2020](https://www.gdcvault.com/play/1026613/).

## 3. Budget the whole obligation, including interactions

Use a scope ledger rather than a list of attractive features. For each candidate,
record its player benefit, required content, dependencies, validation work,
maintenance cost, and what could replace it more cheaply.

| Cost category          | Questions to answer before committing                                               |
| ---------------------- | ----------------------------------------------------------------------------------- |
| Initial implementation | Can we make the basic behavior work? What remains unknown?                          |
| Interactions           | What existing behaviors must work with it? What happens in conflicting states?      |
| Content                | How many assets or scenarios use it? Who creates and revises them?                  |
| Communication          | How does a player discover, understand, and control it?                             |
| Integration            | What changes in saving, input, UI, performance, audio, or build packaging?          |
| Completion             | What testing, accessibility, compatibility, release preparation, and support apply? |

An illustrative example: adding an elemental attack is not just an attack effect
if it changes enemy behavior, scenery, status icons, tutorials, saves, and
encounter balance. Conversely, an extra encounter using established rules may
be a smaller obligation. Neither is automatically the better design choice.
Inspect the actual dependency graph; do not claim every feature multiplies all
other features or use an invented universal growth formula.

Derek Yu warns that menus, transitions, saving, and repetitive content work are
easy to overlook during the exciting conception stage. His _Finishing a Game_
essay treats finishing as a skill and recommends maintaining a playable core
that can survive cuts. [Yu, “Finishing a Game,” page dated September 17; year not
displayed in the reviewed text](https://makegames.tumblr.com/post/1136623767/finishing-a-game).

**Proposed estimation method:** measure a representative unit all the way through
revision and integration, then use a range for the remaining units. Include
separate fixed work and explicit uncertainty. Do not infer total production
speed from the most enjoyable coding session. A second unit helps expose what
is repeatable; it does not guarantee every later unit costs the same.

Protect capacity for learning from playtests and completing the release. Decide
the reserve from observed work and uncertainty; no fixed percentage is justified
by this research. When the work no longer fits, reduce the planned obligation
before silently relying on faster future work or longer hours.

## 4. Use different gates for different questions

Rami Ismail distinguishes design prototypes, which test whether a game should
be made, from a vertical slice, which tests whether the team can produce it.
He recommends a representative segment near intended quality and a second
content unit to assess the revised production process. This distinction is more
useful than treating every impressive demo as evidence of readiness to scale.
[Ismail, September 26, 2022](https://ltpf.ramiismail.com/prototypes-and-vertical-slice/).

The following gates are an adapted operational framework. They are not a mandate
to spend months on formal preproduction for a tiny game.

| Gate              | Deliverable                                                                                | Evidence required to move forward                                                        | If evidence is missing                                     |
| ----------------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Design experiment | Cheapest playable or otherwise testable representation of the uncertain appeal             | Intended experience appears; the main risk becomes clearer                               | Change the hypothesis, simplify, or stop                   |
| External playable | A build someone else can start and use                                                     | Players can reach the important experience without constant explanation                  | Fix communication, controls, or the underlying interaction |
| Production slice  | A representative portion at credible intended quality, followed by another comparable unit | Actual production and revision costs fit capacity                                        | Reduce content, fidelity, dependencies, or the concept     |
| Whole-game draft  | Rough path through the complete intended experience                                        | Beginning, progression, resolution or session boundary, and required transitions connect | Finish the missing path before expanding breadth           |
| Release candidate | Packaged build and accurate release materials                                              | Declared requirements pass on supported configurations                                   | Fix blockers or revise promises and scope                  |

An isolated movement toy can answer a control question but cannot validate a
long narrative arc or a multiplayer relationship. A beautiful scene can answer
an art-direction question but cannot establish the pace of making a world.
Choose fidelity according to the uncertain experience rather than a rule that
every prototype must look crude.

Before each experiment, write: **“We will test X by showing Y to Z; we will
continue if we observe A, revise if B, and stop if C.”** Set a time/effort limit
you can afford to lose, with no automatic extension.

For example, in a hypothetical routing game, ask whether players deliberately
trade safety for speed and can explain the consequence. Continue if that choice
is present and appealing; revise if it is obscured by poor feedback; pause if
the central choice remains uninteresting after the agreed experiments. Do not
respond to a failed hypothesis automatically by adding progression, crafting,
or a larger map. If a new system is the proposed remedy, state the new test and
its cost explicitly.

## 5. Maintain a playable rhythm

Tom Francis describes a prototype as the smallest playable proof of what is
good about the game, including art-led or narrative-led appeal. He recommends
making an internal prototype ready for outsiders next, testing before content
completion, and allowing time to respond. He cautions that a small initial
sample can misrepresent the eventual audience. [Francis, January 8, 2026](https://www.pentadact.com/2026-01-08-15-years-of-indie-dev-in-4-bits-of-advice/).

**Proposed routine:** at each working checkpoint, produce a build, play the
current experience from its normal entry, and record the most important
remaining obstacle. Use a cadence suited to the project; “daily” or “weekly”
is a planning choice, not evidence of an optimal universal interval.

Keep each active task connected to one of three outcomes: resolve a named
uncertainty, improve an observed player problem, or complete a release
requirement. Keep a small active queue so new ideas do not continually interrupt
finishing existing work.

For external tests:

1. Specify what the test needs to reveal and recruit people relevant to that
   question, including fresh players when testing onboarding.
2. Let them start and play without a designer's running explanation. Intervene
   when necessary, but record the intervention as evidence of a problem.
3. Observe behavior before proposing solutions: where did they hesitate,
   misunderstand, disengage, or intentionally try something again?
4. Ask what they thought happened and what they hoped to do next. Separate their
   experience from requested features.
5. Select a bounded response, then test again. A suggestion becomes a hypothesis
   to evaluate, rather than a new release commitment.

Do not treat a handful of enthusiastic friends, social-media likes, or a polished
trailer as equivalent to independent players using the game. Equally, avoid
turning testing into a requirement for an arbitrary number of respondents that
your project cannot recruit. State sample limitations and increase evidence
as the investment and audience claims increase.

## 6. Cut obligations while protecting the promise

Make an ordered cut list before the schedule becomes desperate. This proposed
order is a starting point, subject to the game's defining experience:

1. Remove duplicated experiences and low-value variations.
2. Reduce quantities: places, encounters, branches, modes, or supported targets.
3. Replace bespoke content or exceptional rules with an established pattern.
4. Remove a dependent subsystem and its associated content, UI, and testing work.
5. Reconsider the core promise if the surviving complete game still does not fit.

Do not assume that cutting implementation time is the same as reducing remaining
effort. A partly built feature might still demand substantial content, balancing,
or support; a removal might break progression. Evaluate **future** work saved
and the effect on the complete player experience, not how painful it feels to
discard past work.

Factorio provides a useful case from an ambitious systemic game: Wube's 2020
release plan describes canceling a campaign, postponing fluid changes, and
reducing a GUI rewrite. These independent cuts and progress on other work
enabled an earlier 1.0 date. The source explicitly says the cuts were not caused
by the competing game's launch date. It also reserves a finalization period and
freezes localization changes. [Wube, May 29, 2020](https://www.factorio.com/blog/post/fff-349).

The practical lesson is to define what this release must accomplish even when
the wider game remains ambitious. “We can improve this later” is only a useful
cut if the current version remains complete, understandable, and honestly
presented. Do not depend on a promised update to repair an invalid first release.

## 7. Control technical ambition and the urge to restart

Yu describes developers repeatedly rebuilding their opening levels as their
skills improved, expanding laterally without finishing. He recommends getting
the whole game playable before most polishing and seeing the current project
as one of many opportunities to create. He also acknowledges that aesthetic-led
games can depend more heavily on polish. [Yu, “Indie Game Dev: Death Loops,”
last updated December 14, 2022](https://www.derekyu.com/makegames/deathloops.html).

**Proposed engineering rule:** approve a rewrite, framework, editor, or generic
system only against a concrete blocked task, recurring measured cost, or
required quality risk. Compare it with the smallest local repair. Record the
expected benefit, effort limit, migration cost, and the date to evaluate it.
“This will be cleaner” alone is insufficient evidence that it advances release.

This does not justify unreliable state, lost saves, broken builds, or code that
makes ordinary changes impossible. Stabilize those foundations. Keep new
architecture proportional to behavior actually required by the agreed game;
avoid designing an engine for several hypothetical future games during this one.

Keep an ambition parking lot containing the idea, why it is exciting, what it
depends on, and the smallest experiment that could evaluate it later. Review it
at a deliberate scope checkpoint. Parking an idea is not a promise to implement
it in the current game or its updates.

## 8. Define done in player and release terms

Write a finish line that a build can pass:

- A new player can start, understand enough to play, reach the promised
  experience, and reach the intended conclusion or session boundary.
- Required saving, restarting, recovery, controls, settings, and presentation
  work for the actual kind of game function as intended.
- Known release-blocking failures are resolved on declared supported targets;
  remaining issues are understood and acceptable against an explicit severity
  policy.
- Distribution, credits, asset rights, store materials, and a way to receive
  support reports are ready where applicable.
- Public descriptions match the contents of this build.

These are proposed acceptance categories, not a complete platform compliance
checklist. For a Steam release specifically, Valve reviews both store presence
and build, expects startup on listed supported operating systems, and requires
listed supported features to exist in the build. Check the current requirements
for the actual platform before committing the release date. [Steamworks Review
Process, accessed October 18, 2026](https://partner.steamgames.com/doc/store/review_process?l=english).

Once this definition is met, require a specific blocker or evidenced benefit to
reopen scope. Classify late work as a release blocker, a bounded improvement, or
a later possibility. Avoid treating every visible imperfection as a blocker.

Shipping frequently is not a universal solution either. Sokpop's January 2021
post says its two-week release pace was achievable but left little room for
updates or larger side projects; it proposed switching to monthly releases.
That post is a proposal, not evidence here of the eventual change or a claim
about today's schedule. The useful constraint is a sustainable finish-and-support
cycle. [Sokpop Collective, January 4, 2021](https://www.patreon.com/sokpop/posts/future-of-45796860).

## 9. Warning signs and immediate decisions

The following are proposed diagnostic triggers, not statistical predictors:

| Warning sign                                                    | Decision to make now                                                                 |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| The next build is always postponed for infrastructure           | Name the experiment or release task being blocked; choose the smallest enabling work |
| The core will become enjoyable only after several more systems  | Isolate and test that dependency before expanding production                         |
| New features enter faster than complete experiences emerge      | Freeze additions and complete the current path                                       |
| The opening gets remade while the ending remains a note         | Build the whole-game draft and reassess its shape                                    |
| Content estimates rely on unmeasured future speed               | Finish and revise another representative unit; reforecast                            |
| Every playtest request becomes a task                           | Identify the underlying observed problem and compare possible solutions              |
| Cuts remove the very thing players were promised                | Change the release promise explicitly or pause the project                           |
| “Almost done” has no finite acceptance list                     | Write and prioritize the remaining release requirements                              |
| The work fits only by consuming all contingency or support time | Cut now, revise the release date, or reconsider capacity explicitly                  |

## Evidence limits

These sources mainly reflect experienced independent developers and selected
projects. Successful releases are more visible than abandoned work; firsthand
advice still contains hindsight and survivorship bias. Cook supplies a failed
prototype case, but this is not a representative study of project outcomes.
Deadlines, team arrangements, audience access, and funding vary substantially.

The research supports a practical direction: reduce uncertainty before expanding
commitments and make completion visible. It does not prove a maximum safe number
of mechanics, ideal team size, fixed prototype duration, required playtest count,
or universal reserve percentage. Nor does it imply strategic depth is the goal
of every game. Narrative, atmosphere, social play, and aesthetic experiences need
their own evidence of appeal.

Use the workflow to decide the next experiment and the next complete build.
For a specific concept, the most useful follow-up would be a one-page scope
agreement, a risk-first prototype question, a measured content unit, and an
ordered cut list based on that game's actual promise and capacity.
