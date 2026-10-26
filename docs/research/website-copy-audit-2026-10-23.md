# Website copy audit

Researched 2026-10-23. Scope: repetition and stock AI phrasing in the landing page;
preserve its brief, end-user focus and established layout.

## Evidence and its limits

### GPT-6.1 Sol

OpenAI calls the model **GPT-6.1 Sol**. Its [model
page](https://developers.openai.com/api/docs/models/gpt-6.1-sol) describes its
capabilities, but supplies no measured inventory of its writing habits. A search
of official documentation for the model with “writing” and “style” did not uncover
one. This is a search result, not proof that no such evidence exists.

The [GPT-6 writing
guidance](https://developers.openai.com/api/docs/guides/latest-model#personality-and-writing-style)
discusses detailed formatting, recurring expressions, stock transitions, rhetorical
question/answer pairs, contrast formulas, and invented compound labels. It advises
plain words and direct statements. The guide explicitly attributes the observed
behavior to **GPT-6 Astra**, with prompts offered as starting points for other
models. Its examples include “delve,” “foster,” and “leverage.” These are useful
editorial warnings; they are not documented Sol-specific findings.

### General LLM prose research

[Kobak et al., _Delving into LLM-assisted writing in biomedical publications through
excess vocabulary_](https://arxiv.org/html/2406.07016v5) (2025) examined 15.1 million
English PubMed abstracts from 2010–2024. After ChatGPT's release, excess vocabulary
was predominantly stylistic verbs and adjectives, unlike earlier topic-driven
changes. Examples include forms of _delve_, _underscore_, and _showcase_. The
authors explicitly say corpus-level evidence cannot identify individual
AI-assisted abstracts or distinguish which models influenced them. This supports
checking unnecessary flourishes, not treating ordinary words as proof of AI use.

[Zamaraeva et al., _Comparing LLM-generated and human-authored news text using
formal syntactic theory_](https://aclanthology.org/2025.acl-long.443.pdf) (ACL 2025)
compared English NYT-style writing from six LLMs with human writing. Their
grammatical analyses found greater similarity among model outputs and greater
diversity in human writing. The study did **not** confirm every proposed style
marker, including a claim about participial modifiers. Findings depend on genre,
prompts, and the models studied. They do not measure GPT-6.1 Sol or landing-page
copy. The practical inference is to inspect repeated sentence construction as
well as word choice; no isolated clause or punctuation mark establishes authorship.

## Local findings before this pass

These observations concern the existing copy, not a claim about its authorship.
The source text lives in [Story.tsx](../../web/src/Story.tsx),
[Features.tsx](../../web/src/components/Features.tsx),
[SiteFooter.tsx](../../web/src/components/SiteFooter.tsx), and
[Document.tsx](../../web/src/Document.tsx).

| Repetition                                                                                                               | Editorial consequence                                                                                                                                               |
| ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sharing appears in the hero slogan, lead, final chapter label, annotation description, and metadata.                     | Several lines restate the same purpose. Keep the product explanation where it first clarifies the app, then let later copy describe the next action.                |
| Images recur across the hero, upload explanation, batch description, annotation description, and comparison description. | Later sentences can often omit the object when the heading and context already establish it. Do not replace it with vague words such as “content” just for variety. |
| The final story heading and body both name copying.                                                                      | Let the heading carry the action and the body give the destination or result.                                                                                       |
| Open source appears as a heading and again in the footer, beside an MIT license link.                                    | The footer can be shorter without losing the license destination.                                                                                                   |
| Several small feature headings use the same short, punctuated beat.                                                      | Individual fragments suit the page; uniform cadence across every tile can feel manufactured. Evaluate the sequence aloud.                                           |

Accessibility status text and descriptive artwork alternatives serve different
purposes from visible promotional copy. Do not sacrifice their clarity to improve
a whole-file keyword count. Navigation and download labels may reasonably repeat.

## Editorial decisions for this page

These rules reflect the owner's requested voice and this audit's judgment; the
research above does not establish them as universal writing laws.

- Delete a duplicated thought before reaching for a synonym. Replacing _share_
  with _deliver_, _distribute_, or _publish_ can distort what the app actually does.
- Give each line a job. A heading and its following sentence should contribute
  different information.
- Use concrete verbs. Avoid generic promises about elevating workflows,
  unlocking possibilities, seamless experiences, or effortless productivity.
- Avoid staged questions, unsolicited “not X, but Y” contrasts, grand metaphors,
  and three-item benefit lists added for rhythm. They cost words without adding
  useful product meaning here.
- Keep brief labels and fragments when they work. Do not make every section a
  sequence of emphatic fragments, or turn every feature into a slogan.
- Mention video, annotations, and comparison once where they belong. Do not add
  tool inventories, supported-format lists, implementation details, or promises
  about future documentation.
- Preserve precise product names and necessary repeated control labels. Judge
  the page in reading order, not by a target vocabulary score.

## Applied pass

The hero and browser title now read “GitHub, illustrated.” The lead keeps the
single explanation of images and videos. The upload chapter uses “Choose a
repository, then drop in your files.” The final chapter's heading names copying;
its body now gives the next action: “Paste the Markdown into your conversation.”
The feature row uses “Add several files at once.” and “Annotate before uploading.”
alongside the existing “Compare two images.” The footer keeps its MIT license
link and drops its repeated open-source sentence. Artwork descriptions and
essential control labels retain their separate accessibility purposes.

Counting the visible copy across all three story chapters, chapter navigation,
features, download section, and footer, with inflected forms grouped:

| Word family     | Before | After |
| --------------- | -----: | ----: |
| share / sharing |      4 |     2 |
| image / images  |      5 |     2 |
| video / videos  |      3 |     1 |
| copy            |      2 |     1 |
| open source     |      2 |     1 |

These counts exclude metadata, image alternatives, hidden headings, and live
status announcements. They document this edit, not a target or automated test.
