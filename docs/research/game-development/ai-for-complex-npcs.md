# Using AI to develop complex, engaging NPCs

Research date: 2026-10-18. Scope: dialogue and social character design for a small indie game, independent of rendering style, genre, or engine. This is research and a proposed design approach, not a game implementation plan for this repository.

## Finding

Use AI primarily as a character-development collaborator, rehearsal partner, and testing assistant. Build the game around characters whose competing goals, incomplete knowledge, relationships, and past experiences cause observable changes in what they say **and do**. Consider live language generation only where it adds a specific kind of player expression that authored dialogue cannot provide economically.

The useful target is **recognizable motives with surprising consequences**. A character should sometimes surprise the player, but the player should be able to understand the decision afterward. Generating different sentences or randomly changing personality does not establish depth.

This recommendation is a synthesis of the primary sources below. Those sources establish useful architectures, authoring techniques, and limited experimental results. They do not establish that runtime LLMs reliably produce better NPCs over a whole commercial game, or that there is a universal recipe for interesting characters.

## 1. Separate three uses of AI

| Approach                         | What AI does                                                                                                    | What ships                                                   | Why consider it                                                                         | Main limitation                                                                                          |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Authoring assistance             | Proposes motivations, challenges contradictions, rehearses scenes, drafts alternatives, finds missing reactions | Human-approved writing and ordinary game logic               | Makes an ambitious cast more tractable; supports offline play and editorial control     | Generated ideas can converge on clichés; editing and testing remain work                                 |
| Offline generation and selection | Generates candidate storylets, reaction variants, or test trajectories; a person curates them                   | A finite, reviewed content library selected using game state | Adds variation while retaining testable content, localization, and predictable delivery | More variants can create more continuity and coverage problems                                           |
| Runtime language or agent system | Interprets free text, proposes responses or actions, or generates speech during play                            | A model service or local model plus supporting game systems  | Useful when negotiating, improvising, or expressing player intent is central to play    | Reliability, delay, operating cost, privacy, moderation, and consistency become ongoing product concerns |

These approaches can be combined. An NPC can have a sophisticated social model and entirely authored speech. An LLM can interpret player input while the response remains authored. A runtime model can phrase a refusal without deciding whether the NPC refuses.

**Default for a small team:** start with authoring assistance and state-dependent authored content. Make runtime generation earn its inclusion through player tests.

## 2. What the primary evidence actually supports

### Classic systems: depth existed before LLMs

**Façade — authored dramatic material with adaptive sequencing.** Mateas and Stern describe reactive joint dialogue behaviors organized into story beats selected by a drama manager. The system coordinates moment-to-moment responsiveness with a dramatic structure. This is evidence that authored content and AI selection can work together; it is not evidence that every character needs a full autonomous mind. The content and behavior authoring were substantial. [Paper, AIIDE 2005](https://ojs.aaai.org/index.php/AIIDE/article/view/18722).

**Versu — characters choose within social situations.** Evans and Short describe social practices, such as a meal or a conversation, that offer actions to participants in particular roles. Individual agents choose using their own preferences. Practices can coexist; character definitions are separate from the social situations. Their architecture also makes internal state inspectable and explains emotions through their causes. This is an architectural case study, not a controlled enjoyment comparison. [Authors' architecture paper, especially sections 3–5 and 9–11](https://versu.com/wp-content/uploads/2014/05/versu.pdf); [peer-reviewed system paper](https://cs.uky.edu/~sgware/reading/papers/evans2014versu.pdf).

**Prom Week / Comme il Faut — make relationships playable.** The designers model social knowledge, relationships, statuses, traits, and social history, with social exchanges changing that state. This shifts the interesting variation from which sentence gets played to how an exchange changes future possibilities. The system represents an authored simplification of social life, not a general psychological simulation. [McCoy et al., Social Story Worlds with Comme il Faut](https://mtreanor.com/publications/TCIAIG-CiF.pdf); [Prom Week paper, AIIDE 2013](https://ojs.aaai.org/index.php/AIIDE/article/view/12662).

**Design implication:** borrow the separation between character, situation, choice, consequence, and performance. Reproducing these research systems in full would be excessive for most indie projects.

### LLM agents: useful ingredients, persistent weaknesses

**Generative Agents (2023).** Park et al. combine experience records, retrieval by relevance/recency/importance, reflection, and planning in a 25-agent town. Comparisons with versions missing architectural components favored the full system in interview-rated believability; a two-simulated-day run produced information diffusion and coordination. The authors also observed retrieval errors, invented embellishments, overly formal speech, and excessive cooperation that could erode characters' own interests. This supports investigating memory and goal continuity, while protecting the ability to refuse. It does not establish enjoyable conversation across weeks of player sessions; the simulation was resource-intensive under its historical setup. [Paper, sections 4, 6–8](https://arxiv.org/html/2304.03442v2).

**Character-LLM (EMNLP 2023).** Shao et al. train role-playing models from character experiences and evaluate them through interviews. This explores stronger characterization than a short persona prompt, but interview fidelity to a role is a different outcome from compelling game dialogue, causal world interaction, or sustainable production costs. Fine-tuning is a later option when a stable cast and evidence of a specific model failure justify it. [Paper and evaluation](https://aclanthology.org/2023.emnlp-main.814/).

**CharacterBox (2024 preprint).** Its authors argue that isolated question answering misses how characters behave through unfolding situations, and propose evaluation in text-based virtual worlds. Treat this as a useful direction for trajectory-based testing, not proof that its metrics predict player enjoyment or that simulated evaluators replace players. [Paper](https://arxiv.org/html/2412.05631v1).

### Recent player studies: freedom creates opportunities and friction

**Towards Enhanced Immersion and Agency for LLM-based Interactive Drama (ACL 2025).** The authors report benefits from playwriting guidance and plot reflection in a constrained adaptation: three scenes, eight NPCs, ten human players and ten aggressive LLM players, producing sixty histories across three architectures, with six annotators. They also document future-plot leakage and an invented location. This is promising small-scale evidence for narrative scaffolding, with substantial limits on generalization. [Final paper, sections 4–6](https://aclanthology.org/2025.acl-long.546.pdf).

**Office Whispers (2026).** An exploratory study with 48 participants, four NPCs, and roughly twenty-minute play sessions found appreciation for expressive freedom alongside frustration with generic, inconsistent, or uninformative dialogue. Apparent lies sometimes interested players, but the NPCs had not been programmed to lie: accidental invention should not be mistaken for designed deception. The convenience sample and combined VR/voice/LLM setup do not isolate individual causes or demonstrate long-term retention. [Author-institution full text, especially discussion and limitations](https://publikationen.bibliothek.kit.edu/1000190720/175162755).

**The Double-Edged Sword of Open-Ended Interaction (2026 preprint).** A 130-participant prototype study reports increased autonomy alongside more cognitive load and lower usability/trust, without a significant overall experience advantage. Its single-session setting and inconsistent descriptions of the experimental design warrant caution. Use it as a warning against equating open input with a universally better experience. [Preprint](https://arxiv.org/abs/2604.10107).

**Interpretation:** a model that talks fluently can still obstruct the game. More freedom is valuable when players understand what they can attempt and the game can resolve those attempts meaningfully.

## 3. Define character complexity through decisions

The following is a recommended authoring model, not a validated psychological taxonomy. Model only dimensions that create content or affect play.

| Layer                     | Questions for the writer                                                                               | How the player can perceive it                                          |
| ------------------------- | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------- |
| Stable values             | What does this person protect? What would make them ashamed? What boundary will they usually maintain? | Consistent refusals, selective generosity, recognizable judgments       |
| Conflicting desires       | What do they want that conflicts with another thing they want?                                         | Hesitation, bargains, costly choices, partial cooperation               |
| Immediate pressure        | What needs to happen today? What are they afraid will happen next?                                     | Urgency, diverted attention, initiating or ending a conversation        |
| Knowledge and belief      | What have they witnessed, heard, misunderstood, or deliberately concealed?                             | Different accounts, reasonable mistakes, changes after evidence         |
| Relationships             | Who is trusted, resented, admired, feared, or owed something? Are these feelings reciprocated?         | Public/private differences, alliances, favors, third-party consequences |
| History                   | What did the player actually do, and what did this NPC notice?                                         | Specific callbacks, changed access, commitments remembered later        |
| Strategy and presentation | How do they try to get what they want? How does their public self differ from their private one?       | Evasion, humor, flattery, understatement, confrontation                 |
| Capacity for change       | Which experiences can change a belief, habit, or allegiance? Which cannot?                             | Gradual or event-driven change with an understandable cause             |

A long biography is useful only if parts of it can influence decisions, voice, or interpretation. A single friendship endangered by a concrete promise can create more playable depth than pages of unexpressed lore.

Avoid making every NPC secretly traumatized, deceptive, or involved in a conspiracy. Ordinary pride, affection, obligations, fatigue, competence, and incompatible loyalties can supply conflict. Allow some people to be direct, kind, or uncomplicated in some situations. Contrast across the cast matters.

Do not reduce every relationship to one affection score. Someone may respect the player's competence, distrust their discretion, and owe them a favor. Start with the few distinctions the writing actually uses. Asymmetry matters: admiration does not imply reciprocal admiration.

## 4. Use AI to develop the NPCs before writing lots of dialogue

### Step 1: The human sets the dramatic purpose

Write a short brief for each major character: role in the game, current predicament, two competing desires, one boundary, important relationships, what they know, and the kinds of change their arc permits. Decide what is distinctive about the person before asking for polishing.

GamePlot provides relevant, limited evidence for this collaborative approach. Its study with 14 game designers found appreciation for generating, playing through, and revising narrative material while retaining control and ownership. Participants also identified limits in nuanced, complex, and innovative writing. This is evidence about an early design tool and designers' experiences, rather than proof of improved finished NPCs or player retention. [Authors' paper, user study and discussion](https://arxiv.org/html/2411.02714v1).

Keep canon, provisional ideas, and rejected ideas separate. An attractive model suggestion must not quietly become a fact that later prompts assume is approved.

### Step 2: Generate competing interpretations, then choose

Ask AI for materially different reasons behind the same observed behavior. For example: the ferryman refuses a passenger. Perhaps he fears for them, protects another passenger, resents being commanded, or has promised someone to keep the boat empty. These imply different future actions; four versions of the same angry line do not.

An authoring prompt to adapt:

> Given the approved facts below, propose three different explanations for this character's refusal. Each must preserve those facts, conflict with a second thing the character wants, and imply one costly action they might take later. Identify the cliché risk in each. Do not add a secret, tragic history, or new world fact unless you mark it as a proposal.

Select an explanation because it serves the cast and game, not because it sounds elaborate. Combine only when the result remains legible.

### Step 3: Build a behavior matrix

Give AI the same request under changed conditions: public versus private, trusted player versus stranger, calm versus time pressure, weak evidence versus proof, friend present versus rival present. Ask for the **decision and its cause** before dialogue.

Use this to expose flatness. If the NPC always helps after a friendly line, a biography has not produced a character with priorities. If every condition produces a completely different person, the priorities are unstable. Keep several recognizable habits across contexts.

### Step 4: Rehearse scenes as adversarial collaboration

Have AI play both sympathetic and difficult interlocutors: a quiet player, an impatient player, a literal player, someone who repeats themselves, someone who bargains well, and someone who tries to flatter away a boundary. Rehearse the NPC pursuing their own agenda while responding to each.

Ask specifically for contradictions between stated beliefs and actions, unearned intimacy, exposition disguised as conversation, repeated metaphors, and moments where the NPC becomes an agreeable assistant. A separate critique pass can be helpful, but a second model's approval is still not independent evidence that players will enjoy it.

### Step 5: Draft at the level of conversational action

Ask for alternatives that accomplish different things: admit one fact while avoiding another, request proof, offer a compromise, change the subject, make a demand, withdraw a previous offer. Then ask for a few voice variants of the chosen action.

Emily Short's first-party account warns that tiny word substitutions can produce awkward prose with little gameplay value; she recommends variation tied to state and conversational action. Her broader conversation-design writing emphasizes contextual continuity and NPC initiative. These are experienced design judgments, not controlled comparisons. [Conversation as Gameplay, especially the Restless discussion](https://emshort.blog/2019/01/20/conversation-as-gameplay-talk/); [Conversation](https://emshort.blog/how-to-play/writing-if/my-articles/conversation/).

Keep important lines under human editorial control. Distinctive rhythm, humor, subtext, and well-timed silence deserve revision. AI can propose; the writer decides what the character would actually say.

### Step 6: Turn the best exchanges into reusable content

Save small scenes with conditions, intent, effects, repeat behavior, and fallback content. Review the result inside the actual game context. Dialogue that reads well in a transcript may fail when it interrupts movement, repeats a discovery, or arrives after the player already acted.

There is experimental evidence for both help and homogenization: Doshi and Hauser's short-story study found that AI ideas improved evaluated story quality, particularly for some writers, while AI-assisted stories became more similar to one another. This does not directly measure NPC production. Use it as a reason to keep human taste and diverse starting material active, rather than assume more generated candidates automatically mean a more distinctive cast. [Author paper](https://arxiv.org/abs/2312.00506).

## 5. Design conversation as a sequence of consequential actions

Each substantial exchange should have a reason to exist beyond delivering a paragraph. It can change understanding, establish a promise, alter trust, expose a conflict, close an opportunity, or make another action possible. Quiet companionship and humor are valid purposes too; every line need not advance a quest.

Recommended mechanics to prototype:

- **Requests with tradeoffs.** The NPC may agree reluctantly, agree conditionally, refuse, or counteroffer for reasons the player can infer.
- **Evidence and interpretation.** A fact can be accepted, doubted, reinterpreted, or concealed. Let the player use what they learned elsewhere.
- **Public and private stakes.** A character may help in private but resist the same request in front of a rival. Make the audience visible.
- **Commitments over time.** Remember promises as actionable obligations, including who made them and when they can be fulfilled or broken.
- **NPC initiative.** An NPC follows up, asks for help, raises an uncomfortable issue, or leaves for a prior obligation. Avoid making every character wait indefinitely for questions.
- **Repair after failure.** A bad encounter can create a different route, a delayed apology, or a new alliance. Preserve the significance of failure while avoiding accidental dead ends.
- **Selective repetition.** Give a concise recap when needed; react to persistent pressure only when that reaction is intended gameplay. Provide a journal or recap so forgetting a clue does not force repetitive questioning.
- **Room to stop.** End at a useful beat. There should be a satisfying answer to “why return later?” other than more generated words.

Emily Short distinguishes selecting content, managing available options, resolving actions, and presenting results. She notes that unlocking a future option can communicate consequence without requiring a wholly separate story branch. That is a useful production pattern for a small team. [What does your narrative system need to do?](https://emshort.blog/2022/04/09/what-does-your-narrative-system-need-to-do/).

Choose the input method for the interaction you want. Authored choices can convey subtle player voice and make available actions clear. Free text can support improvisation and unusual phrasing, but asks players to invent their own moves and asks the game to interpret them. A hybrid can show a few reliable actions alongside optional free text. Unsupported attempts should receive clear, context-appropriate feedback rather than silently being treated as a different intent.

## 6. Example: an NPC who becomes interesting through competing loyalties

This is an invented design example. The following lines are authored illustrations, not quotes or reported model outputs.

**Mara repairs the town's flood machinery.** She wants everyone kept safe and wants her younger brother Ivo to retain his apprenticeship. She knows he bypassed an inspection. She initially believes exposing him would end his career without fixing the system that encouraged the shortcut. Her former mentor Sen can arrange repairs but distrusts Mara's judgment.

Mara is proud of her competence, dislikes public humiliation, and refuses to falsify a safety certificate. She will conceal a name; she will not certify a dangerous machine. These are distinct boundaries.

| Situation                                                                     | Mara's response                                             | Consequence                                                                       |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Player asks who skipped inspection in front of Ivo                            | Deflects toward getting the mechanism fixed                 | Ivo knows the player is investigating; trust in the player's discretion may fall  |
| Player brings a damaged part in private                                       | Acknowledges danger and asks to examine it                  | Opens a practical repair route; no automatic confession                           |
| Player asks her to sign a false safety certificate while blaming someone else | Refuses the false certificate, even if she likes the player | Reveals a value the player can rely on                                            |
| Player earns Sen's cooperation without humiliating Ivo                        | Accepts a supervised repair and discloses Ivo's role        | Repair access changes; the brother's future depends on a later accountable action |
| Player threatens public exposure                                              | May stop protecting the player and seek Sen directly        | A plausible alternative alliance forms; intimidation has a continuing cost        |

First meeting:

> “If you've come for a culprit, join the queue. If you've brought a spanner, come inside.”

After the player brings evidence privately:

> “You brought this to me first. I noticed. Now show me where it cracked.”

When asked to sign the false certificate:

> “I'll defend him at the hearing. I won't put my name on that machine.”

The surprise is not that she suddenly has a random new personality. It is that the person who obstructed an investigation becomes the person who insists on an honest safety finding. Both choices follow her values under different pressures.

AI is useful here for proposing believable compromises, finding where the boundaries conflict, drafting voices, and rehearsing hostile encounters. The game should own whether the certificate exists, whether repairs are possible, who witnessed the threat, and what Mara actually did. Generating a persuasive sentence must not create a new repair route or erase the threat.

## 7. A practical architecture, if runtime AI is justified

The following is a proposed implementation boundary, not a claim that a particular research system guarantees reliability.

1. **Record the situation.** The game supplies current scene, participants, relevant objects, recent events, available actions, and conversation state.
2. **Select what this NPC can know.** Retrieve only this character's observations, received information, beliefs, and permitted disclosures. Keep inaccessible secrets out of their input where possible.
3. **Interpret player intent.** Use ordinary UI actions or a constrained language-model classification. Preserve ambiguity; ask an in-world clarification or expose the interpreted action before a high-stakes commitment.
4. **Choose and resolve an action.** Game rules determine eligible actions and effects. A small utility system, state machine, storylet selector, or bounded planner can prioritize current goals. If a model proposes an action, the game validates it against the same rules.
5. **Select or generate expression.** Present an authored line for critical facts, promises, quest outcomes, and major emotional moments. A model can optionally express an approved lower-stakes conversational act using permitted facts.
6. **Check and present.** Validate structure, identifiers, length, and explicit permissions. Content checks can flag problems but cannot guarantee that free prose contains no unintended promise, implication, or spoiler. For guarantees, use authored text or constrained templates.
7. **Commit consequences once.** Apply game-authorized effects in one transaction. Record the event and who observed it. Make the same event available to later dialogue and gameplay.
8. **Recover cleanly.** On timeout, refusal, malformed output, or failed checks, use an authored fallback that reflects the same resolved action. Reject stale responses when the scene has changed.

Keep the chosen action, its game effects, and its spoken presentation separate. A line that says “take the key” must correspond to an actual authorized transfer, or be explicitly designed as a deception the game understands. Do not let arbitrary generated strings execute commands.

### Memory with provenance

Maintain an authoritative event record and a smaller character-facing memory view. Each important remembered fact should identify its source: witnessed event, statement from another character, inference, or authored belief. Track whether the NPC believes it, whether it is true in the world, and whether the player knows it. These can differ.

Pin facts whose omission would break play: outstanding promises, current commitments, key boundaries, and essential knowledge. Summaries can support retrieval, but do not let summarization overwrite the original events or silently turn a suspicion into a fact. For a small cast, tagged records and explicit rules may be enough; a vector database is an option to justify through retrieval needs.

Model deliberate forgetting or distortion only if it serves the character and the player can understand it. Accidental retrieval failure is a bug, not character depth.

### Believable surprise

Give variation a limited role among eligible, motivated actions. Preserve strong boundaries, authored revelations, and important prerequisites. Use uncertainty in what the player knows about the character, competing priorities, and changing relationships before adding randomness.

When a surprise occurs, leave evidence of why: a witness, a previous promise, a visible deadline, or a later explanation. Developer tools should expose the selected action and state that caused it. An eloquent after-the-fact explanation from the model does not prove that the system selected the action for that reason.

### Authored content remains a serious option

Ink supports conditional choices, variables, visit counts, and controlled text variation. Yarn Spinner supports conditional storylets and line groups with content-selection strategies; its documentation includes ways to favor less-seen content. These provide mechanisms for contextual dialogue without a runtime LLM. They do not supply an engaging character model automatically. Choose one that fits the eventual engine and workflow, rather than adopt both. [Writing with ink](https://github.com/inkle/inky/blob/master/app/resources/Documentation/WritingWithInk.md); [Yarn Spinner saliency](https://docs.yarnspinner.dev/write-yarn-scripts/advanced-scripting/saliency); [current storylet example](https://yarnspinner.dev/docs/godot/gdscript/05-connecting-to-your-game/06-saliency-and-storylets/).

## 8. A small indie MVP

These are suggested experiment sizes, not measured industry optima or a development-time estimate.

Build **three connected NPCs in one location, around one recurring conflict**. Give each two competing desires, one reliable boundary, a relationship to each other character, and a small set of consequential memories. Author a short initial encounter, a changed encounter after a player action, and a later payoff for a promise or betrayal.

Implement a compact set of social actions: request, offer, disclose, challenge, and withdraw, for example. Make at least one action outside the conversation affect a later exchange. Show one NPC changing a plan without waiting for the player. Let one disagreement be repairable and one boundary remain firm.

First make this work using AI-assisted writing and authored responses. Then add exactly one runtime capability to compare: free-text intent interpretation, or generated delivery of already-resolved lower-stakes acts. Adding both at once makes it harder to identify why the experience changes.

The MVP succeeds when players remember specific people and moments, can explain why a relationship changed, and want another encounter. A long transcript, high response diversity, or a model's favorable self-evaluation is insufficient.

Do not start with a continuously reasoning town full of independently chatting agents. Update off-screen characters when relevant events or scene changes occur. Concentrate simulation and writing on interactions the player can perceive or later discover.

## 9. Evaluate whether the NPCs are interesting after novelty wears off

Treat the following as a proposed evaluation plan. Existing short studies are not a substitute for testing this game's audience and pacing.

### Compare the right baselines

Keep the same cast, world facts, encounter, and writing budget where practical. Compare:

1. A strong authored baseline with state and consequences.
2. That baseline plus the proposed AI capability.
3. A targeted ablation, such as removing specific event memory or flattening conflicting goals.

Do not compare a polished AI version only against a deliberately weak fixed script. To isolate wording from underlying behavior, hold decisions and effects fixed while changing authored versus generated delivery. To isolate social simulation, hold delivery quality similar while changing whether history affects outcomes.

Run repeated visits and delayed returns. Counterbalance presentation order where feasible, and distinguish participants who enjoy improvising from those who prefer clear objectives. Record player behavior and interview responses as well as ratings.

| Question                     | Useful evidence                                                             | Failure to watch for                                          |
| ---------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Is the person distinctive?   | Players identify the speaker from unlabelled lines and explain priorities   | Every NPC shares the same warm, explanatory voice             |
| Is the person coherent?      | Players explain surprising choices using earlier evidence                   | Motivations change to accommodate the latest request          |
| Is talking worthwhile?       | Players voluntarily return, pursue an unresolved issue, or change a plan    | High word count and politeness with little reason to continue |
| Does history matter?         | A specific action changes a later option, refusal, or alliance              | Generic “I remember you” with no behavioral difference        |
| Does the player have agency? | Intent maps to a visible consequence, including intelligible failure        | The prose celebrates a choice the game ignores                |
| Is variation substantive?    | Different strategies lead to meaningfully different social outcomes         | Synonym changes conceal identical decisions                   |
| Is challenge fair?           | Players can discover why an approach failed and try a plausible alternative | Opaque mood rolls, keyword guessing, or hidden prerequisites  |
| Is it sustainable?           | Return visits remain interesting within measured cost and delay             | First-session novelty masks repetition or exhausted content   |

Collect examples of memorable moments, frustration, repeated topics, unsupported intents, contradictions, and fabricated affordances. Use human review for humor, subtext, emotional impact, and whether a player understood the consequence. Automated checks can reliably inspect state invariants and missing references; model judges can triage transcripts but should not decide the overall verdict.

Stress cases should include repeated requests, flattery, threats, silence, contradictory testimony, topic changes, returning after a long gap, a witness leaving, an NPC already dead or absent, a broken promise, a loaded save, and network failure during an exchange. Re-run selected histories against model or prompt changes; preserve model/version, scenario state, content version, and response logs needed to investigate regressions. Temperature zero alone does not guarantee reproducible behavior across service changes.

## 10. Runtime costs, pacing, and product boundaries

### Latency is part of the conversation

Measure time to first usable content, full turn completion, player interruption/cancellation, and the slow tail on target hardware and network conditions. Keep replies proportionate to the scene; test reading rhythm, not just backend speed. Streaming makes content visible sooner but can expose unchecked text, so decide whether a line must be validated before display.

A 2026 study of 34 participants using a particular Llama 3 8B setup reports token-generation latency affecting experience more than the initial delay. Only publisher abstract/snippets were available for this review. It does not establish a universal acceptable pause or token rate for other games. [Publisher abstract](https://www.sciencedirect.com/science/article/pii/S1875952126001357).

Precompute likely low-stakes material when the state is stable, but invalidate it when relevant facts change. Put a firm timeout and authored recovery path behind every live request. Keep controls usable while waiting, and do not fake a character response that implies a consequential action before that action is resolved.

### Budget the complete pipeline

Estimate cost per completed player session using **all** input/output tokens and calls: intent interpretation, response generation, retrieval if model-assisted, summarization, off-screen simulation, retries, moderation, and evaluation. Multiply by realistic conversation length, session count, and active-player volume. Measure cache effectiveness rather than assume it. Put hard budgets on background work and runaway conversations.

An indicative accounting identity is:

`session inference cost = sum over calls(input tokens × input unit price + output tokens × output unit price) + other service charges`

Use current provider terms and prices for a later implementation decision. Historical paper costs are not estimates for a 2026 product. Local inference trades service dependence for hardware requirements, downloads, battery/thermal pressure, and performance variability; measure those on the intended minimum specification.

### Privacy, prompt attacks, content, and continuity

- Free text can contain personal information. Decide what is sent off-device and retained, communicate that plainly, minimize collection, and give players workable controls for deletion and opting out where the design supports it.
- Keep service credentials out of distributed clients. Authenticate and rate-limit any game backend that pays for model use.
- Treat player text, retrieved memories, and generated prose as untrusted input. They cannot authorize changes to inventory, relationships, hidden facts, or system rules. Schema validation checks structure; it does not guarantee semantic correctness.
- Give each NPC only the information needed for the encounter. A prompt saying “do not reveal the murderer” still exposes the identity to the model if it is present in context.
- Establish an intended content range and recovery responses. A character can be abrasive, evasive, or stubborn without turning the interaction into uncontrolled harassment or a generic out-of-character service refusal.
- Save the consequential conversation history needed for continuity. Do not regenerate important past utterances after loading and assume the new wording means the same thing. Separate optional diagnostic logs from the game's persistent story record.
- Make the designed experience survive unavailable services and rejected outputs. Authored alternatives should preserve facts and consequences, including refusal and uncertainty.

These are engineering and product recommendations, not a complete security review or jurisdiction-specific legal guidance.

## 11. What to borrow from authored narrative games

Sam Barlow's GDC session on _Her Story_ explicitly frames writing for the player's imagination as a design goal. The session overview was verified; the complete talk was not reviewed here. The transferable proposal is to let players infer a person from partial, conflicting evidence instead of having the NPC explain their full psychology on demand. [First-party GDC session](https://www.gdcvault.com/play/1023430/Making-Her-Story-Telling-a).

ZA/UM writer Justin Keenan's GDC session on _Disco Elysium_ discusses the cost and value of difficult dialogue and narrative choices. Only the session overview was inspected, so no detailed implementation claim is attributed to it. Treat it as a next reading/viewing source, not proof that a particular AI architecture recreates that game's writing. [First-party GDC session](https://www.gdcvault.com/play/1027048/contactUs).

The practical recommendation is to preserve authored specificity: a precise observation, an unfair judgment, an awkward joke, a refusal that costs the speaker something. Use AI to explore and test those choices, then select and revise them. A system's capacity to continue indefinitely is different from an author's ability to choose the next worthwhile moment.

## 12. Recommended decision

For the stated goal, develop **a small cast with interlocking motives, observable memories, and consequences**, using AI heavily during design, rehearsal, drafting, and testing. Ship the first version with reviewed, state-dependent dialogue. Use authored storylets or equivalent content units to make return visits responsive without an uncontrolled branching explosion.

Add runtime AI only after identifying a valuable missing interaction—such as negotiating an unusual compromise in the player's own words—and demonstrate that it improves repeated play over the authored baseline. Retain an authoritative game state and authored treatment of critical facts, decisions, and payoffs.

The core production question is: **can a player encounter this person again, do something different, and discover a believable new side of them that changes what happens?** That is a stronger test of depth than whether the NPC can generate a new answer to every question.

## Source register and evidence limits

Links were checked during this review. Publication dates below identify the cited work; a living documentation page may change later. Peer review, public code, and first-party design commentary provide different kinds of evidence and should not be treated as interchangeable.

| Source                                                                                                                                                                                                                  | Type / date                                  | Reviewed scope                                   | Use in these notes                                                     |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------- |
| [Mateas & Stern, Structuring Content in the Façade Interactive Drama Architecture](https://ojs.aaai.org/index.php/AIIDE/article/view/18722)                                                                             | Research paper, 2005                         | Publisher abstract                               | Authored behaviors, beats, drama management                            |
| [Evans & Short, The AI Architecture of Versu](https://versu.com/wp-content/uploads/2014/05/versu.pdf)                                                                                                                   | Author-hosted architecture paper             | Full text, selected sections                     | Character/situation separation, inspectable state, social practices    |
| [Evans & Short, Versu—A Simulationist Storytelling System](https://cs.uky.edu/~sgware/reading/papers/evans2014versu.pdf)                                                                                                | Peer-reviewed paper, 2014                    | Abstract and architecture discussion             | Autonomous choices within social practices                             |
| [McCoy et al., Social Story Worlds with Comme il Faut](https://mtreanor.com/publications/TCIAIG-CiF.pdf)                                                                                                                | Author-hosted research paper, published 2014 | Full text, social-state architecture             | Relationships and social history as game state                         |
| [McCoy et al., Prom Week](https://ojs.aaai.org/index.php/AIIDE/article/view/12662)                                                                                                                                      | AIIDE paper, 2013                            | Publisher abstract                               | Game applying CiF                                                      |
| [Park et al., Generative Agents](https://arxiv.org/html/2304.03442v2)                                                                                                                                                   | UIST paper, 2023                             | Full text, architecture, evaluation, limitations | Memory/reflection/planning and their limitations                       |
| [Shao et al., Character-LLM](https://aclanthology.org/2023.emnlp-main.814/)                                                                                                                                             | EMNLP paper, 2023                            | Abstract and full-paper evaluation               | Character training and interview-based fidelity                        |
| [CharacterBox](https://arxiv.org/html/2412.05631v1)                                                                                                                                                                     | Preprint, 2024                               | Abstract and evaluation framing                  | Assessing unfolding character trajectories                             |
| [Towards Enhanced Immersion and Agency for LLM-based Interactive Drama](https://aclanthology.org/2025.acl-long.546.pdf)                                                                                                 | ACL paper, 2025                              | Full-paper evaluation and limitations            | Narrative scaffolding and leakage risks                                |
| [Office Whispers study](https://publikationen.bibliothek.kit.edu/1000190720/175162755)                                                                                                                                  | Journal article, 2026                        | Author-institution full text                     | Expressive freedom versus unreliable progress                          |
| [The Double-Edged Sword of Open-Ended Interaction](https://arxiv.org/abs/2604.10107)                                                                                                                                    | Preprint, 2026                               | Full paper, with method-reporting caveat         | Cognitive load and mixed experience findings                           |
| [Latency study, Entertainment Computing, DOI 10.1016/j.entcom.2026.101213](https://www.sciencedirect.com/science/article/pii/S1875952126001357)                                                                         | Journal article, 2026                        | Publisher abstract/snippets only                 | Pacing needs measurement; no universal thresholds                      |
| [Doshi & Hauser, Generative artificial intelligence enhances creativity but reduces the diversity of novel content](https://arxiv.org/abs/2312.00506)                                                                   | Author paper; journal publication 2024       | Abstract                                         | Ideation benefits and similarity concern; nongame experiment           |
| [Alavi et al., Game Plot Design with an LLM-powered Assistant](https://arxiv.org/html/2411.02714v1)                                                                                                                     | Author preprint, 2024                        | Abstract, design workflow, user-study discussion | Iterative authoring with human control; limited N=14 designer evidence |
| [Emily Short, Conversation](https://emshort.blog/how-to-play/writing-if/my-articles/conversation/)                                                                                                                      | First-party design essay                     | Full text, selected sections                     | Context, agency, initiative, repetition                                |
| [Emily Short, Conversation as Gameplay](https://emshort.blog/2019/01/20/conversation-as-gameplay-talk/)                                                                                                                 | First-party retrospective, 2019              | Full text                                        | Stateful variation, legibility, consequences                           |
| [Emily Short, What does your narrative system need to do?](https://emshort.blog/2022/04/09/what-does-your-narrative-system-need-to-do/)                                                                                 | First-party design essay, 2022               | Full text                                        | Separate choices, resolution, and presentation                         |
| [Writing with ink](https://github.com/inkle/inky/blob/master/app/resources/Documentation/WritingWithInk.md)                                                                                                             | Official living documentation                | Conditional-choice and variation sections        | Authored implementation option                                         |
| [Yarn Spinner saliency](https://docs.yarnspinner.dev/write-yarn-scripts/advanced-scripting/saliency) and [storylets](https://yarnspinner.dev/docs/godot/gdscript/05-connecting-to-your-game/06-saliency-and-storylets/) | Official living documentation                | Selection strategies and examples                | Authored implementation option                                         |
| [Barlow, Making Her Story](https://www.gdcvault.com/play/1023430/Making-Her-Story-Telling-a)                                                                                                                            | First-party GDC session, 2016                | Overview only                                    | Player imagination; further viewing                                    |
| [Keenan, Disco Elysium: Meaningless Choices and Impractical Advice](https://www.gdcvault.com/play/1027048/contactUs)                                                                                                    | First-party GDC session, 2021                | Overview only                                    | Further viewing, no detailed architecture claims                       |

No vendor demonstration is used as evidence of long-term enjoyment, production reliability, or economic feasibility. No source reviewed here settles how much runtime autonomy this particular game should have; that remains an experiment for the proposed small cast and intended players.
