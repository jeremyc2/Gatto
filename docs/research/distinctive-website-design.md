# A distinctive, playful website for Gatto

Research date: October 16, 2026 (America/Chicago). Scope: visual identity, narrative, interaction, accessibility, and the visitor's route to downloading. This is research and a proposed decision process, not an approved visual direction or an implementation plan.

## The useful starting point

Build the identity around **a small, capable cat helping an image get where it belongs**. That supplies a subject, an action, and a personality. A palette and a collection of attractive effects supply none of those on their own. This is a design hypothesis drawn from the app's existing identity and behavior, not a research finding that one particular cat aesthetic will convert best.

The design task has two concurrent goals: someone in a hurry must understand the app and reach its release page; someone curious should find a memorable little world. Put the essential explanation in ordinary reading order and make the extra play voluntary. Distinctiveness can live in composition, illustration, motion, language, and relationships between them without making navigation unfamiliar.

Absolute uniqueness against every website cannot be demonstrated. A more useful brief is: **recognizable as this app, coherent across situations, memorable after use, and easy to understand without instruction**. A still image, a muted animation, a narrow layout, and a keyboard visit should all retain that identity.

## AI-era sameness: what the evidence actually supports

### Older defaults and the newer defaults that replaced them

A November 12, 2025 first-party design article identifies recurring combinations of ordinary typography, purple gradients on white, predictable layouts, and little motion. It also acknowledges that instructions to avoid one familiar typeface can make generation converge on another. This is practitioner evidence with selected examples, not a representative audit of the web. [Design-guidance article](https://claude.com/resources/articles/improving-frontend-design-through-skills)

More recent practitioner guidance, pinned to its September 2, 2026 revision, names these additional recurring treatments:

| Observed treatment                                                                    | Question to ask in our design review                                         |
| ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Cream, prominent serif lettering, clay accents                                        | Does this world have a reason to feel printed or earthy?                     |
| Near-black with acid-green or vermilion accents                                       | Does this express our character, or just signal fashionable intensity?       |
| Newspaper columns and fine dividing rules                                             | Does the information actually benefit from editorial density?                |
| Identical rounded cards, shadows, gradient washes                                     | Which information deserves grouping, and which belongs directly on the page? |
| Repeated uppercase labels, small monospaced captions, ornamental numbering and arrows | What information does each mark communicate?                                 |
| Repeated fade-and-slide entrances                                                     | What change is this movement helping the visitor understand?                 |

The left column summarizes the [September 2026 guidance](https://github.com/anthropics/claude-code/blob/dbdd79cebfae5891f5b0fab6f7773ea520d289a7/plugins/frontend-design/skills/frontend-design/SKILL.md); the right column is our proposed review method. These are observed defaults, not measured popularity rankings or reliable ways to identify who made a page.

This means “avoid purple” is an inadequate brief. A replacement palette can become equally interchangeable. Nor should Gatto discard its existing cyan-violet illustration accents solely because a generic gradient is familiar. A color earns its place through its relationship to the character, content, hierarchy, and surrounding visual language. That is a design judgment, not a claim that particular hues are uniquely human or uniquely AI-generated.

### Why polishing the first answer is insufficient

A March 2026 preprint investigates website-design homogenization through a literature review, walkthroughs of six creation systems, and qualitative risk analysis. It proposes deliberate reflection and contextual anchoring as mitigations. It does **not** measure the prevalence of individual visual tropes or experimentally establish that its proposed mitigations improve distinctiveness. Its useful contribution here is a diagnosis of the workflow risk: accepting the first plausible result can surrender the very decisions that make work specific. [Shin et al., March 13, 2026](https://arxiv.org/html/2603.13036v1)

A separate CHI 2024 experiment with 60 participants found greater fixation and fewer, less varied, less original ideas in the AI-image-assisted condition than its baseline. The task concerned avatar ideation, not marketing websites; it supports caution about anchoring, not a universal claim about current models. [Wadinambiarachchi et al., 2024](https://arxiv.org/abs/2403.11164)

A March 24, 2026 first-party experiment also reports that aesthetic evaluation language itself produced convergence: asking for museum-quality work pushed outputs toward a particular visual character. Better internal scores were not always the author's preferred designs. This is an exploratory account, not independent proof of a universal mechanism. [Design-evaluation experiment](https://www.anthropic.com/engineering/harness-design-long-running-apps)

**Our inference:** positive, subject-specific decisions matter more than a growing blacklist. Compare genuinely different concepts before refining one. Evaluate clarity and memorability with people; an automated opinion about its own originality is insufficient evidence.

### A practical audit for this project

The following are proposed review exercises, not validated scientific tests:

1. **Remove the name.** Can someone still describe the app's particular world and purpose? A mascot pasted onto an otherwise interchangeable page is a weak result.
2. **Replace the content.** Could an unrelated service occupy the same composition without substantial redesign? If so, make the illustration, hierarchy, demonstration, or narrative more specific.
3. **Explain five decisions.** Why this headline rhythm, illustration style, accent, grouping shape, and movement? “It looks modern” is too vague to govern a coherent system.
4. **Compare silhouettes.** View the candidates small and without legible words. Different colors on the same arrangement are one concept with variations.
5. **Remove motion.** The visual identity, explanation, and download path should survive intact. Animation should add character and show cause and effect.
6. **Audit the words.** Replace interchangeable promises such as “unlock your potential” with a concrete description of what the visitor can do and what they receive.

Originality cannot be certified against every website. The achievable goal is a recognizable identity, grounded in this app, that intended visitors can understand and remember. Familiar navigation and clear action labels can coexist with an unusual visual world.

## What the evidence actually supports

| Finding                                                             | Evidence and limitation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Design implication: an interpretation                                                                                                                   |
| ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Visual complexity and familiarity affect first impressions quickly. | Two experiments varied screenshot complexity, typicality, and exposure time; lower complexity and higher typicality were rated more appealing. These were short screenshot judgments, not marketing conversion tests. [Tuch et al., 2012](https://research.google/pubs/the-role-of-visual-complexity-and-prototypicality-regarding-first-impression-of-websites-working-towards-understanding-aesthetic-judgments/).                                                                                                                                                                                                                                                                                                             | Keep the information hierarchy legible while making the subject and execution specific. Novel artwork need not mean novel rules for finding a download. |
| Beauty does not repair poor interaction.                            | An 80-participant experiment varied appearance and usability of a shopping experience. Poor usability reduced subsequent beauty ratings; visual beauty did not determine perceived usability in that experiment. [Authors' first-party account, 2012](https://research.google/blog/is-beautiful-usable-what-is-the-influence-of-beauty-and-usability-on-reactions-to-a-product/).                                                                                                                                                                                                                                                                                                                                                | Observe whether people succeed, rather than accepting “looks great” as evidence of usability.                                                           |
| Concise, scannable, factual writing has experimental support.       | The original web-reading research compared rewritten content, with study materials published for replication. Its reported improvements belong to that study and era; they are not a forecast for this site. [Original research](https://www.nngroup.com/articles/concise-scannable-and-objective-how-to-write-for-the-web/), [replication materials](https://www.nngroup.com/articles/downloadable-files-to-replicate-web-reading-study/).                                                                                                                                                                                                                                                                                      | Give headings meaningful information and write concrete outcomes. Keep playful language around the explanation rather than inside indispensable labels. |
| Different game elements can produce different experiences.          | A simulation experiment analyzed 331 participants after completion and awareness checks. Element bundles affected competence or relatedness differently; decision freedom did not significantly change. Bundling and exclusions limit generalization. It did not test download conversion. [Sailer et al., 2017, author-uploaded paper](https://www.researchgate.net/profile/Michael-Sailer-2/publication/311879391_How_gamification_motivates_An_experimental_study_of_the_effects_of_specific_game_design_elements_on_psychological_need_satisfaction/links/5874ebdc08ae329d62202795/How-gamification-motivates-An-experimental-study-of-the-effects-of-specific-game-design-elements-on-psychological-need-satisfaction.pdf). | Start with feedback, agency, and a satisfying result. Adding points, badges, and a mascot does not establish that the experience is fun or useful.      |
| Motion can make a page inaccessible or physically uncomfortable.    | The accessibility guidance distinguishes automatic movement from interaction-triggered animation and explains vestibular risks. [Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html), [Pause, Stop, Hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html).                                                                                                                                                                                                                                                                                                                                                                                              | Design a complete still experience alongside motion, and give visitors control.                                                                         |
| Speed, responsiveness, and stability are distinct qualities.        | Published experience thresholds address main-content loading, interaction latency, and unexpected layout movement, assessed at the 75th percentile of visits. They do not measure visual originality or all accessibility. [Threshold rationale](https://web.dev/articles/defining-core-web-vitals-thresholds).                                                                                                                                                                                                                                                                                                                                                                                                                  | Judge delight by its effect on the visit, including slower visits, rather than only its appearance in an ideal recording.                               |

The following sections are a synthesis and proposed design rules. Numeric starting points for palettes, type styles, and visual variation are editorial constraints, not scientific constants.

## Begin with material that belongs to this app

The [overview](../../README.md), [walkthrough](../WALKTHROUGH.md), and [artwork rationale](../../packaging/dmg-artwork.md) establish useful raw material: a rocket cat, a dark illustrated world, luminous cyan-blue with violet edges, and a practical image-to-copyable-result workflow. The app accepts images in several ways, supports batches and descriptions, and offers a separate faster clipboard workflow. Those are product facts; the visual metaphors proposed below are new interpretations.

Make an inventory before gathering a mood board:

- **The visitor's unfinished job:** an image is ready, but getting it into a conversation still involves friction.
- **The app's job:** help turn that image into a ready-to-paste reference.
- **The character:** capable, helpful, energetic, slightly mischievous. Avoid making the cat clumsy merely to manufacture jokes.
- **Existing visual material:** the cat's anatomy, face, arc of travel, matte arrow, smooth illustration, and controlled light.
- **The emotional result:** a small piece of work is finished; the visitor can get back to the conversation.

Collect non-website references for composition, character acting, movement, lighting, and lettering. A reference should answer a specific question—how a small character conveys confidence, for example—rather than become a page to copy. Mark which qualities are transferable and which would misrepresent the app. This is a proposed creative process, not evidence that borrowing from another medium guarantees originality.

## Three coherent directions to investigate

These are mutually distinct hypotheses for exploration. They are not three skins to combine, and no direction has been selected by this research.

| Direction                                   | Narrative and composition                                                                                                                                         | Color, type, and shape relationships                                                                                                                                                                                                                                                               | Interaction signature and risk                                                                                                                                                                                                                       |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A small delivery through the night**      | An image makes a short journey accompanied by the cat. One continuous illustrated route connects a few meaningful moments, with calm reading space beside it.     | Deep blue surfaces, clear cyan route, restrained warm destination light. Soft character contours contrast with simple, sturdy text and controls; rounded forms come from the character and image objects. Light mode becomes a bright overcast scene with dark ink and a darker cyan action color. | A visitor starts a brief delivery demonstration; the route resolves into a completed reference. Risk: scenery may dominate or suggest the website itself uploads images. Clearly label the demonstration and keep it optional.                       |
| **The cat's image workshop**                | The page is an orderly workspace where the cat helps prepare, describe, and send an image. Overlapping image sheets explain the actual transformation.            | Cool pale work surfaces and dark ink by day; layered blue-gray surfaces by night. One saturated working accent, with a little warm light for completion. Image-sheet corners and soft feline silhouettes determine the shape vocabulary.                                                           | Selecting a sample brings it to the work area and reveals its result. Risk: a fake desktop can add unnecessary controls. Keep the lesson short and show only actions relevant to the explanation.                                                    |
| **A tiny flight with a useful destination** | Extend the existing rocket-cat identity: an image takes one decisive arc from input to destination. Generous negative space makes the trajectory the composition. | Navy and bright sky versions share a clear silhouette. Cyan-violet belongs to the exhaust and trajectory; action controls use a consistent solid treatment. A compact expressive display style is balanced by quiet, readable body text.                                                           | An explicitly started launch shows input, transfer, and completion; the final state remains readable. Risk: a space spectacle can become generic or exaggerate speed. Give the flight a specific object and purpose, without numerical speed claims. |

The first and third have the most direct continuity with existing artwork. The workshop may explain the product most literally. Choose based on comprehension, recollection, and the quality of the whole system, not the strongest isolated hero image. Keep any concealed in-app discovery concealed; this research does not propose publishing its entry action or turning it into a marketing requirement.

## Make a design language with reasons

Write a one-page identity brief before polishing screens. A useful draft could be: “A small capable companion makes image sharing feel finished. The world is playful; the information is direct. Movement follows a purpose and settles.” This is a candidate, not final copy.

Translate it into decisions that reinforce one another:

| Decision                              | Reason within that brief                                            | Review question                                                      |
| ------------------------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------- |
| One dominant visual gesture           | A journey, handoff, or preparation is the central action.           | Does the page have one recognizable compositional idea?              |
| Controlled expressive typography      | The character carries personality; instructions remain calm.        | Are important words readable before decorative details?              |
| A small family of related contours    | Character, image, and destination share a visual world.             | Can each unusual curve or corner be explained by the same premise?   |
| Space that groups related information | The visitor needs to understand the action at a glance.             | Are relationships clear without drawing a box around every sentence? |
| Light with a narrative role           | Attention goes to a route, working object, or arrival.              | Does brightness have meaning, or is everything glowing?              |
| One motion grammar                    | Anticipation, purposeful movement, and settling express capability. | Do different interactions feel governed by the same physical rules?  |

Start with a few text roles: a display headline, section heading, body, and supporting label. Decide the width, density, alignment, emphasis, and rhythm of each. An unusual type choice cannot compensate for anonymous copy. On narrow screens, preserve meaning and character through hierarchy and illustration framing rather than shrinking a poster until it becomes unreadable.

Define shape roles instead of choosing a favorite radius everywhere: ordinary controls should be stable and easy to identify; image containers should read as image objects; an expressive silhouette can belong to the scene. List intentional exceptions. Repeated explanation of exceptions is a sign that the underlying direction is too vague.

## Pick colors by job, then tune the hues

There is no universal correct number of colors. As a starting constraint, explore **one dominant neutral family, one primary chromatic family, and at most one supporting expressive family**, with separate functional colors where needed. A family can contain several lightness steps; this does not mean three literal swatches. Illustrations can contain more colors without making every color a control color.

Assign these roles before selecting final values:

| Role                        | Purpose                                             | Rule to retain across light and dark                                                    |
| --------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Page and raised surfaces    | Establish depth and reading areas.                  | Surface hierarchy remains understandable without relying on an almost invisible shadow. |
| Main and supporting text    | Explain, then qualify.                              | Supporting information remains readable; “secondary” does not mean faint.               |
| Primary action              | Identify the download route.                        | Keep its role and prominence consistent even when its shade changes.                    |
| Secondary interaction       | Support demonstration, navigation, and preferences. | Visitors can distinguish it from both body text and the primary action.                 |
| Focus and selection         | Show location and current choice.                   | Use a clear shape or indicator as well as color.                                        |
| Success, warning, and error | Explain an actual state.                            | Pair color with a word, icon, or other distinguishable cue.                             |
| Expressive light            | Give the illustrated world its character.           | Decorative glow does not become the only way to understand information.                 |

Test the palette on actual compositions: the smallest supporting text, a button, a focused link, a screenshot caption, and an error. A swatch sheet alone cannot show whether those relationships work. Normal text generally needs 4.5:1 contrast; qualifying large text needs 3:1, subject to the criterion's exceptions. [Contrast Minimum](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum). Essential visual indicators for controls and meaningful graphics generally need 3:1 against adjacent colors, with specified exceptions. [Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast). Information must not depend on color alone. [Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html).

Treat light and dark as two art-directed versions of the same place. Keep hue relationships, character silhouette, action hierarchy, and reading rhythm recognizable; change actual lightness and saturation as needed. Inverting every value is a weak starting point for luminous illustration or screenshot framing. Respect the visitor's preference and offer an understandable override; that is a recommended experience policy, not a claim that one scheme is universally more accessible.

The website's two schemes must not imply the desktop app supports appearances it does not. The current [overview](../../README.md) describes the app itself as dark-only. Present authentic interface images in either website scheme.

## Tell the product story in the order a visitor needs it

This proposed sequence is a content hypothesis to test, not a guaranteed conversion formula:

1. **Recognize the job.** A plain sentence explains what the app helps someone do. A character or scene adds feeling without replacing the explanation. The release-page action is visible immediately.
2. **See the result.** Show a real input and the resulting image reference in a familiar conversation context. Visitors should understand the transformation from the still state alone.
3. **Understand the effort.** Show the normal workflow accurately: bring an image in, upload, then copy. Explain the faster clipboard route separately if included; do not suggest that normal uploads automatically replace the clipboard. [Walkthrough](../WALKTHROUGH.md).
4. **See useful range.** Select a few consequential capabilities, such as several images together and editable descriptions. Use compositions that demonstrate them instead of a repetitive grid of decorative feature boxes.
5. **Resolve the download decision.** Put compatibility, installation expectations, current release information, and a setup route near the action. Keep any measured release figure tied to its verified release rather than hardcoding an evergreen claim. The current ownership and measurement rules are in [documentation upkeep](../agents/documentation.md).
6. **Leave a clear next step.** Repeat the release-page action after the explanation. A compact help route supports visitors who need more detail; future informational pages can extend this structure.

Use the same action wording each time. If the action opens a release page rather than starting a file transfer, its nearby wording should make that expectation clear. Do not insert an artificial sign-up, quiz, game, or animated waiting room between the visitor and that destination. Avoid invented praise, counters, or speed claims. These are trust and clarity recommendations, not measurements of their conversion impact here.

## Make it feel alive through response and agency

A lively page does not require every element to move. Consider three layers:

- **Feedback:** the selected sample is unmistakable; a control acknowledges activation; a demonstration reports completion.
- **Explanation:** a single object moves to show the relationship between input and result.
- **Optional personality:** the cat reacts once to a completed visitor action, or a small scene detail can be explored voluntarily.

The strongest initial candidate is an optional, short **sample-image delivery demonstration**. Show the starting image and finished example even before interaction. A visitor may start, pause, replay, or skip the animation. Label it as a demonstration so it cannot be mistaken for a real upload or actual download. Make the result explanatory, not merely a celebration.

Game-like feeling can come from a comprehensible action, responsive feedback, a reachable conclusion, and a small reward in the character's behavior. It need not include scores or competition. Treat “the cat celebrates a successful handoff” as a testable design hypothesis. The motivational study above does not prove that this character behavior increases downloads.

Give each proposed animation a job, trigger, end state, and still equivalent. For example: “On selecting Play, the image follows the illustrated route, settles at the destination, and leaves the result visible; the still version shows both objects with the same labels.” If its only justification is that a page otherwise feels boring, reconsider the composition or content first.

Avoid mandatory dragging, scavenger hunts for facts, moving targets, unexpected sound, pointer replacement, prolonged introductory sequences, and scroll behavior that takes control away from the visitor. Keep the page's core explanation outside the optional scene. These choices protect the brief's requirement that play not become work.

## Accessibility and performance belong in the design brief

Design the ordinary and reduced-motion experiences together. For reduced motion, remove decorative travel, camera motion, parallax, shaking, and large zooming; preserve state changes, readable results, and optional controls. Interaction-triggered nonessential animation being disableable is a Level AAA criterion; choosing to honor it is appropriate for this brief, but should not be mislabeled as an AA-only obligation. [Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html).

Automatically moving information that runs more than five seconds alongside other content generally needs pause, stop, or hide controls, unless essential. Automatic updating has its own condition without that five-second allowance. Stopping movement only while a visitor holds focus is insufficient. [Pause, Stop, Hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html). Prefer a calm initial state and explicitly started demonstrations rather than treating a five-second duration as an invitation to distract.

Essential website actions must work from the keyboard, with a visible focus indicator that remains discoverable rather than disappearing under a fixed download bar. [Keyboard](https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html), [Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html), [Focus Not Obscured](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html). Design comfortable targets; the AA minimum criterion uses 24-by-24-pixel sizing or specified alternatives and exceptions, rather than requiring every target to be exactly that size. [Target Size Minimum](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html). Core text and functionality should survive 200% enlargement, and ordinary page content should reflow at a 320-pixel-wide equivalent without two-direction scrolling. [Resize Text](https://www.w3.org/WAI/WCAG21/Understanding/resize-text), [Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html). This is a selected design checklist, not an exhaustive conformance audit.

For this project's visual direction, use no strobing, rapid flashes, or flashing celebrations. That is a deliberately simpler design policy than managing the accessibility standard's frequency, area, and brightness exceptions. [Three Flashes or Below Threshold](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html). Plan descriptions for meaningful screenshots and demonstrations, keep purely decorative scenery out of the reading sequence, and verify that the story and result make sense with a screen reader. These are proposed review tasks alongside the visual studies.

For performance, agree on three experience targets: main content appears within 2.5 seconds, interactions respond within 200 milliseconds, and unexpected layout shift stays at or below 0.1, using the published metrics and 75th-percentile interpretation. [Threshold rationale](https://web.dev/articles/defining-core-web-vitals-thresholds). These are evaluation targets, not claims about the future site.

Translate those into visible design choices: present the explanation and download action promptly; keep space for artwork so controls do not jump; let optional scenes start after the page is already useful; keep a complete still presentation available when rich media is delayed. A high automated score is useful evidence about measured checks, but it cannot decide whether the visual identity is distinctive or a visitor understands the promise. Manual task and accessibility reviews remain necessary.

## A manageable research-to-design process

1. **Write the brief and content spine.** One visitor, one job, one promise, one primary destination, a few personality traits, and the facts that must be visible. Remove unsupported claims.
2. **Explore three directions at equal fidelity.** Give each the same headline, actual interface image, download action, feature explanation, and still/reduced-motion moment. That makes differences in the system easier to judge than differences in polish.
3. **Test recognizable identity.** Hide the app name and ask what the design suggests and what someone recalls later. Compare the direction with unrelated app material: if the same imagery and wording fit equally well, it is insufficiently specific. This is a proposed qualitative test, not a validated numerical originality scale.
4. **Test the job.** Without explaining the page, ask someone to tell you what the app does, identify its result, find the download route, and find installation help. Observe hesitation and mistaken expectations. Ask about appearance after the tasks so aesthetic praise does not become the only evidence.
5. **Stress the system.** Review both appearances, a narrow view, enlarged text, keyboard movement, no hover, reduced motion, delayed imagery, and a returning visitor who wants only the download.
6. **Choose one direction and document its grammar.** Record color roles, typography roles, spacing relationships, illustration rules, shape roles, motion purposes, and prohibited shortcuts. Record why they belong together.

Use a compact scorecard when choosing: specificity to the app; comprehension; ease of reaching the release page; coherence across appearances and states; quality of optional play; comfort and access; loading, response, and stability. Agree what “good” looks like before reviewing the concepts. A beautiful direction that fails a core task needs revision even if it wins the visual preference vote.

The first design deliverable should be a small identity brief, three comparable concept studies, a content sequence, and a motion/still storyboard. That is enough to choose a direction without starting implementation or pretending the research has already made an aesthetic decision.
