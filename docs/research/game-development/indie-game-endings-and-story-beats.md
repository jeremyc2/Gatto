# Writing indie game endings and major story beats

Research date and source access date: October 18, 2026 (America/Chicago).

**Working recommendation:** decide what the ending should make the player
understand, feel, or take responsibility for; then build the playable experiences
that earn that response. Draft the whole arc early, including the final action
and its aftermath. Revise it through play, rather than treating an outline as a
finished story.

**Spoilers:** discusses the structure and an escape ending of *Papers, Please*,
the design of *Galatea*, *Alabaster*, and *Floatpoint*, and broad narrative
approaches in *Undertale*, *Hades*, *Outer Wilds*, *Heaven's Vault*, *A Highland
Song*, *SOMA*, and *Scarlet Hollow*. Linked sources may contain larger spoilers.

This guide addresses authored stories in independent games, including small
linear games, branching conversations, exploration mysteries, and repeated-run
games. It does not prescribe a plot for Gatto. **Creator evidence** below means
firsthand writing, interview answers, or an official conference abstract.
**Proposed practice** and the worked example are this note's synthesis, not
claims that every creator follows one formula. Conference abstracts establish
the advertised topic and intent; the talks themselves were not viewed.

## 1. Define what changes, before choosing a plot formula

For planning purposes, a **beat** is a meaningful change in the player's
situation or understanding: a discovery, commitment, reversal, loss, new
capability, or changed relationship. A scene can contain several beats. A
**major beat** changes what the player is trying to do, why it matters, or how
they can do it. A lore fact or impressive cutscene does not automatically make
that change.

Begin with a small set of concrete sentences:

- **Activity:** what the player repeatedly does.
- **Desire:** what the protagonist is trying to obtain, protect, escape, or prove.
- **Contradiction:** how their current approach makes that desire harder to
  achieve, harms someone, or conflicts with another value.
- **Pressure:** what prevents them from carrying on comfortably as before.
- **Ending question:** what the last stretch will settle.
- **Lasting change:** what will be different after the final action.

For a character story, a productive contradiction might be, “She wants her
friends to trust her, but hides every mistake.” Make it observable: she destroys
a failed repair before anyone sees it. “She struggles with trust” gives the
writer much less to stage. A character need not improve; refusal, compromise,
tragedy, and acceptance can also produce an ending. For an exploration mystery,
the important change may be in the player's model of the world rather than in
the protagonist's personality.

**Creator evidence:** Toby Fox says *Undertale*'s story followed its battle
system, and describes supporting nonviolent completion as substantially more
complex than following established combat conventions. That is evidence for a
mechanics-first route to story, not a requirement to start every game that way.
[Fox interview, December 10, 2015](https://www.themarysue.com/interview-undertale-game-creator-toby-fox/).

**Proposed practice:** check the contradiction against the actual verbs. Can
repairing, negotiating, hiding, fighting, arranging, or exploring expose it?
If the story's conflict only exists in dialogue between unrelated tasks, either
connect those tasks to the conflict or deliberately make the dialogue the main
activity. A quiet conversation can be the climax; combat is not obligatory.

## 2. Draft an ending hypothesis, then work in both directions

Write a rough final sequence before polishing the opening. It can change. Its
purpose is to reveal what the rest of the game must establish.

Use three linked parts:

1. **Final pressure:** the central problem becomes impossible to postpone.
2. **Final action:** the player does, chooses, learns, or completes something
   that settles the central question.
3. **Aftermath:** a concrete response shows what that action meant.

Draft these as events, not as emotions you hope the audience will have. Replace
“a bittersweet ending about letting go” with “the player switches off the beacon
that keeps replaying her father's last transmission; afterward, she hears the
other boats calling to one another.” The second version can be staged and
tested. Whether it feels bittersweet remains an open question.

Then work backward: what must the player know, care about, and be able to do for
that action to have weight? Work forward as well: given this character and these
pressures, would they plausibly reach it? If backward plotting requires the
character to become foolish or abandon an established value without cause,
revise the route or the ending.

**Creator evidence:** Emily Short recommends identifying the minimum facts
required to understand an ending, charting their dependencies, and making those
dependencies playable through puzzles, geography, stats, or choice flow. In
*Floatpoint*, access to materials for a diplomatic outcome required experiencing
vignettes relevant to that outcome. She also recommends letting players act on
critical information, including choosing how an inevitable event happens or
how the protagonist responds.
[Short, May 18, 2016](https://emshort.blog/2016/05/18/plot-shaped-level-design/).

**Proposed practice:** keep an ending contract with four fields: the central
question answered; the facts required; the relationship or value tested; the
observable consequence. Separate those from optional world history. An open
ending can leave the future uncertain while clearly showing the choice made
and its immediate cost. Decide which uncertainty is intentional.

## 3. Write major beats as causes and consequences

Instead of starting with “inciting incident, midpoint, climax” as empty boxes,
write the change each beat must produce. Structural labels can help afterward;
they cannot supply the causal connection.

A useful scene sentence is:

> Because of the previous event, the protagonist tries **X**. **Y** resists or
> complicates that attempt. The player **does or decides Z**. This changes **A**,
> so the next attempt must be different.

For example, “She visits the workshop and learns its history” is an information
stop. “She asks the mechanic to repair her father's transmitter; he refuses
because she concealed the damaged battery; she can disclose the damage or
attempt a risky repair herself” creates an immediate conflict. Neither version
is universally better, but the second provides actions, a relationship turn,
and consequences to carry forward.

Give each major beat at least one consequence that survives the scene: a new
goal, debt, belief, route, limitation, promise, or relationship. Repeatedly
raising the volume without changing the situation produces escalation in
presentation only. Conversely, a small act can turn the story if it changes who
trusts whom or what the player now understands.

**Creator evidence:** Lucas Pope describes constructing *Papers, Please* from
pieces such as encounters, characters, documents, puzzles, and story threads,
then arranging them in a grid. His intended progression hooks players early,
develops longer stories in the middle, introduces variation, and builds toward
a climax. He explicitly identifies this as a structure that works for him.
[Pope, May 9, 2013, post #535](https://dukope.com/devlogs/papers-please/tig-06/).

**Proposed practice:** make a beat table with columns for player goal, action,
new information, emotional or relational change, consequence, and prerequisite.
Read just the consequence column. If the same state persists for several major
beats, combine them, change their function, or make the quieter interval an
intentional period of recovery.

## 4. Make setup and payoff playable

For every important ending element, establish more than its existence. Establish
what it normally does, why it matters to someone, and what using it costs.
The finale can then repeat it under changed circumstances.

An original planning pattern:

| Earlier experience | Later pressure | Possible payoff |
| --- | --- | --- |
| A repair always requires disconnecting power first | A distress signal is using that power | The final repair requires an understood sacrifice |
| A companion habitually completes your instructions | The last situation is outside your knowledge | You must ask for their judgment |
| A familiar sound signals a safe route | Its source is revealed to be dangerous | Navigation gains a new meaning |
| You keep returning an object to its owner | The owner finally leaves | Giving it away becomes an ending action |

A twist should change the meaning of earlier evidence. Write the earlier
interpretation, the later interpretation, and the evidence compatible with both.
If the reveal needs a previously unmentioned rule to work, seed that rule or
change the reveal. Surprise is optional; a player anticipating the outcome can
still care deeply about carrying it out.

**Creator evidence:** Thomas Grip's pre-release *SOMA* design pillars call for
puzzles and encounters to arise from narrative situations and for themes to
emerge through participation. This documents an intended approach, not measured
proof of how every player experienced the released game.
[Grip, December 12, 2013](https://frictionalgames.com/2013-12-the-five-foundational-design-pillars-of-soma/).

**Proposed practice:** make a payoff ledger: ending element → earlier encounter
→ player use → changed meaning → fallback if missed. For something essential,
do not rely on one optional diary entry. Provide another appropriate encounter,
gate the relevant outcome on learning it, or write an ending that accommodates
the player's ignorance. Repetition should add context or pressure rather than
repeat an explanation verbatim.

## 5. Choose where agency lives and budget it

Agency can concern the final outcome, the path taken, the interpretation of
evidence, the expression of a personality, or relationships along the way.
Specify which promises your game actually makes. One ending can support rich
participation; several endings can still feel arbitrary.

**Creator evidence:** the official description of Kelsey Beachum's 2021
*Outer Wilds* talk states that the game's primary goal was curiosity-driven,
player-directed exploration, using a nonlinear story instead of assigned
missions. This supports a knowledge-and-discovery form of progression. It does
not establish detailed claims here about the game's final sequence.
[GDC 2021 session abstract](https://www.gdcvault.com/play/1027368/Independent-Games-Summit-Sparking-Curiosity/).

**Creator evidence:** Tony Howard-Arias's 2022 *Scarlet Hollow* session abstract
distinguishes intended discrete endings reflecting players' thematic behavior
from a relationship system using smaller dialogue and art changes to personalize
play. This is a description of the design approach at that time, not a claim
that all planned endings had shipped.
[GDC 2022 session abstract](https://www.gdcvault.com/play/1027639/Game-Narrative-Summit-Making-Player/).

**Proposed practice:** choose a structure for a particular purpose:

| Structure | Useful when | Obligation to watch |
| --- | --- | --- |
| One authored destination | The central experience is mastery, discovery, or a particular dramatic arc | Preserve meaningful participation on the way |
| Local branches that reconverge | Choices primarily express approach or relationship | Remember meaningful differences after reconvergence |
| Several outcome families | Distinct commitments deserve distinct resolutions | Establish each outcome's requirements and consequences |
| Shared finale with conditional aftermath | Many relationships need acknowledgment | Avoid incompatible combinations and generic summaries |
| Repeated runs with persistent narrative | Failure and return belong to the premise | Handle widely different rates of player progress |

Reconvergence does not require forgetting. Inkle's official *Writing with ink*
guide documents branching and rejoining, conditional content, variables, and
visit-based state. These are implementation facilities for remembering earlier
play, not evidence that a particular branch is dramatically meaningful.
[inkle, official ink authoring guide, accessed October 18, 2026](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md).

Write choices in terms of intelligible intentions. “Tell her about the damage”
is clearer than “Be honest” if the actual consequence is a specific disclosure.
Players need not predict every consequence, but they should understand what
they are attempting. If accumulated behavior selects an ending, provide
intermediate reactions so the final result can be traced to play.

Before adding an ending, identify its distinct dramatic meaning. A costume
change may be an epilogue variation; a fundamentally different commitment may
justify another outcome. Budget the scenes, art, audio, prerequisites, save
states, and tests, including combinations. There is no evidence here for an
ideal number of endings.

## 6. Treat pacing as something the player experiences

Outline order is only one clock. Reading speed, retries, exploration, skipped
conversations, and long breaks between sessions change the distance between
setup and payoff. Decide where attention is available and where the player can
pause, reflect, or resume.

**Creator evidence:** Greg Kasavin describes *Hades* as extending *Pyre*'s
continuous narrative, where success and failure both move the story forward.
Characters respond to accomplishments, and players choose how much to engage
with them. This explains an approach to failure as narrative continuity; it is
not a prescription for the number of runs before a finale.
[Kasavin, August 5, 2021](https://blog.playstation.com/2021/08/05/the-origins-of-hades-out-next-week-on-ps5-ps4/).

**Creator evidence:** Jon Ingold's *A Highland Song* development posts describe
context-sensitive remembered correspondence, shaped by what Moira has recently
seen and learned. A later post explains how players can walk away from most
conversations, with reactions and tracking of seen or skipped content. The
design connects story delivery to movement and current context.
[inkle, “Letters,” May 3, 2022, and “Autocut!”, May 11, 2023](https://www.inklestudios.com/highland/).

**Proposed practice:** for each major beat, specify when it becomes eligible,
what competes for attention, what happens if interrupted, and how a returning
player recovers the relevant context. Avoid placing a crucial revelation under
an unfamiliar action challenge unless divided attention is intentional and
tested. Conversely, do not stop a movement-focused game for every minor thought.

Build contrast: effort followed by a view, argument followed by solitary work,
revelation followed by a chance to act on it. A climax need not be the hardest
challenge. Test whether repeated failure at the last obstacle sustains the
intended feeling or replaces it with exhaustion.

## 7. Ending case studies: what the creators actually support

### Papers, Please: presentation can change an ending's meaning

Pope initially intended the Obristan escape to include an interactive border
crossing from the entrant's perspective. He cut it because it demanded work for
one ending, offered limited interaction, created difficult failure/replay
questions, and risked making that ending appear more correct than its peers.
He reused the intro's picture-and-text format instead. In a follow-up, he notes
that the simpler presentation enabled a pointed contrast with Arstotzkan
bureaucracy.
[Pope, November 18–19, 2013, posts #633 and #636](https://dukope.com/devlogs/papers-please/tig-10/).

**Application:** an ending's scale, music, interactivity, and polish can rank
outcomes even when the script calls them equally valid. Audit that implied
judgment. A smaller presentation can preserve the intended meaning and make
finishing feasible; elaborate interaction must earn its place.

Pope also reported twenty endings while explicitly judging that number
excessive; new endings had become a convenient way to resolve different player
actions. This is a useful caution against treating branch count as quality.
[Pope, August 1, 2013](https://dukope.com/devlogs/papers-please/tig-08/).

### Galatea: an ending must fit the conversation that produced it

Short describes the challenge of making *Galatea*'s ending conditions produce
coherent conclusions to varied conversations, and acknowledges that some
generated stories are weaker than others. She also reports players becoming
anxious about whether they had found all endings, despite her intention that
different interests should yield different stories.
[Short, “Multilinear IF,” undated reviewed page](https://emshort.blog/how-to-play/writing-if/my-articles/multilinear-if-older/).

**Application:** test an ending with multiple histories, including a player
whose interests differ from the writer's preferred route. Decide whether the
interface encourages completionism or satisfaction with one personal outcome.
If replay is essential, make that expectation legible.

### Alabaster: failures can teach, and endings need explicit tests

While assembling *Alabaster*, Short charted outcomes beyond the conversation
tree because nonconversation actions also mattered. She marked unimplemented
endings and those with test commands. She proposed that unfavorable endings
offer interesting information and directions for another attempt. In her own
comment, she describes alternating between implementation and charting.
[Short, December 2, 2008](https://emshort.blog/2008/12/02/visualizing-plot/).

**Application:** maintain an outcome map and a reproducible route or saved state
for each ending. For replay-driven stories, failure can resolve the current
attempt while opening a better question. Avoid requiring long repetition that
adds no new understanding.

### Heaven's Vault: revision freedom is a production choice

In a published creator Q&A, Ingold describes rewriting an entire scene late in
development and testing it separately. He and Joe Humfrey connect that freedom
to an authoring system that separates flexible narrative from reusable
presentation, and to not fully voicing or continuously animating every scene.
They describe iterating on story and spatial timing together.
[Ingold and Humfrey Q&A, May 1, 2019](https://www.gamedeveloper.com/design/knockin-on-i-heaven-s-vault-i-how-inkle-designed-its-first-3d-game).

**Application:** preserve the ability to change weak scenes. Choose presentation
and tools with revision cost in mind. Full voice or bespoke animation can be
appropriate, but they create obligations when the ending changes.

## 8. Original worked example: earning a quiet final action

The following miniature game is invented for this guide. It is not a proposed
Gatto plot or a description of a researched game.

**Premise:** a lighthouse apprentice repairs a coastal radio before the last
ferry leaves. Her father recorded a looping message before he died. She wants
to preserve his station exactly, but doing so prevents other people from using
it. The player's verbs are tune, listen, reroute power, repair, and transmit.

**Ending question:** will she keep the station as a private memorial or let it
become a living service? For this small linear version, the authored arc reaches
the latter. The player participates through discovery and execution; it does not
promise an outcome branch.

| Beat | Playable event and scene turn | What it earns |
| --- | --- | --- |
| Familiar routine | Tune the looping recording; the apprentice corrects a loose dial just as her father taught her | The recording is comforting; tuning is learned |
| Disruption | A boat calls for assistance, interrupting the recording; answering requires temporarily switching it off | The same channel has a present-day use |
| First attempt | She tries to run recording and receiver together; the lights dim and the caller breaks up | Her preferred compromise has a visible limitation |
| Relationship turn | A former colleague offers a replacement component; she initially insists her father's setup was perfect | The obstacle has emotional and practical dimensions |
| Reversal | Testing reveals her father's handwritten rewiring notes: he changed the station repeatedly to help others | Preservation and loyalty no longer mean the same thing |
| Cost clarified | A storm disables the safe power supply; the only available compatible part is in the recording unit | The final sacrifice follows established equipment rules |
| Final action | Disconnect the recording unit, move its part, retune, and guide the ferry | Familiar verbs now enact the changed commitment |
| Aftermath | A passenger thanks the station; another boat asks to use it tomorrow; she writes a new entry in the maintenance log | The loss remains, while continued service becomes possible |

The ending is earned by the dependency chain: attachment → competing need →
failed compromise → revised understanding → informed cost → action → evidence
of change. The father is not revealed as secretly malicious, and a speech does
not have to announce the moral. The new log entry can echo her earlier habit of
correcting the dial, now recording her own change rather than restoring his.

**Revision risks:** the replacement must not be a miraculous part introduced
only in the finale. Establish compatibility and the power limitation during
earlier repairs. Do not imply the recording can simply be copied if the dilemma
depends on losing it; either establish the relevant physical constraint or
choose a less contrived cost, such as leaving the station and its recording
behind. If fresh players immediately propose an obvious safe solution, that is
evidence to revise the premise, not proof that they missed the theme.

For a branching version, preserving the memorial needs its own coherent setup,
cost, and aftermath. Do not offer it as an apparently valid choice and then
deliver an insulting two-line punishment. The additional route is a separate
production commitment, not a free dropdown option.

## 9. A compact drafting and revision worksheet

Copy these fields into a story document; fill them with observable events.

### Ending card

- **Central question and answer:**
- **Protagonist's starting desire and contradiction:**
- **What the player understands by the end:**
- **Final pressure and available intentions:**
- **Final player action:**
- **Why earlier play prepares that action:**
- **Immediate cost, consequence, and final image or sound:**
- **Required facts and experiences; how each is encountered or used:**
- **What remains deliberately unresolved:**
- **Outcome conditions, conflicting states, and a route to test it:**

### Major-beat card

- **Entering goal and emotional/relationship state:**
- **Attempt, resistance, and player participation:**
- **New information or reversal:**
- **Persistent consequence and next goal:**
- **Required prior beat; optional discoveries that modify this one:**
- **Setup planted or payoff delivered:**
- **Interruption, failure, retry, and return-after-a-break behavior:**
- **Content cost and the smallest playable version:**

Draft one complete route using placeholder presentation. Play from the ordinary
start; a finale launched in isolation cannot reveal whether its setup is
remembered. Pope's June 2013 account is a concrete reason to do this: his first
whole-game playthrough exposed longer story threads that fell flat even though
the general gameplay progression worked.
[Pope, June 23, 2013, post #572](https://dukope.com/devlogs/papers-please/tig-07/).

During external tests, observe first and ask afterward: “What were you trying
to achieve?”, “What changed your understanding?”, “Why do you think this ended
this way?”, and “What did you expect to happen next?” Ask about a specific
moment rather than inviting a general approval score. Track comprehension,
causal attribution, attention, and feeling separately. A player can understand
an ending and dislike its values; ambiguity and confusion are different results.

Revise the earliest broken dependency. If the final sacrifice feels arbitrary,
more emotional dialogue at the end may not fix the missing earlier attachment.
If the setup is clear but the last interaction feels tedious, revise that
interaction. Preserve enough time and budget to make these changes.

## 10. Pitfalls and deliberate tradeoffs

| Symptom | What to inspect or change |
| --- | --- |
| The ending explains a theme never encountered in play | Add an earlier decision, task, or relationship that puts the value under pressure |
| A twist replaces the story instead of reinterpreting it | Identify the prior evidence and costs that remain meaningful after the reveal |
| A final choice makes earlier choices feel erased | Carry history into eligibility, staging, reactions, or aftermath where the game's promise requires it |
| Every choice opens a permanent new route | Reconverge where outcomes can share a situation; preserve consequential state |
| The player receives the “wrong” ending without understanding why | Improve intermediate feedback and test a causal explanation after completion |
| A sad ending feels like punishment for engagement | Examine the promised experience, player intention, and acknowledgment of effort |
| The climax is followed by a long inventory of minor fates | Prioritize consequences that answer established concerns; let optional detail remain optional |
| Required setup is easy to miss | Change access, reinforce naturally, or adapt the ending to the knowledge actually acquired |
| A beautiful ending exists only as prose | Build its transitions, inputs, failure states, timing, and aftermath in a playable draft |

These are diagnostic proposals, not prohibitions. A game can deliberately deny
control, break a convention, withhold closure, or frustrate a desire. Name the
intended effect and test whether players experience that effect rather than an
accidental broken promise.

## Evidence limits and source selection

The sources document practitioner reasoning and particular production histories;
they are not controlled comparisons of ending quality. Several feature
successful, unusually experienced developers, and text-heavy games are strongly
represented. Their tools and budgets do not establish what one solo developer
can produce. Genre, audience, accessibility, localization, performance skill,
and session length can change the appropriate structure.

Primary material used here consists of creator-authored posts, Pope's own
archive of his original development log, direct creator interview answers, the
official ink manual, and two official GDC abstracts. The *Outer Wilds* slide PDF
was discoverable but returned an access error; no claims depend on its unread
contents. The *Scarlet Hollow* abstract is treated as historical design intent,
not as verification of its October 2026 release state. This note does not claim
to have played all cited games or establish player reception through reviews.

For the most immediately applicable reading, start with Short's
[dependency-based approach](https://emshort.blog/2016/05/18/plot-shaped-level-design/),
Pope's [ending presentation decision](https://dukope.com/devlogs/papers-please/tig-10/),
and the [Heaven's Vault creator Q&A on revision](https://www.gamedeveloper.com/design/knockin-on-i-heaven-s-vault-i-how-inkle-designed-its-first-3d-game).
