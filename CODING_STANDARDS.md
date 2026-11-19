# Coding standards

Review applies every rule here. CI enforces the mechanical rules (`.github/workflows/ci.yml`), so a review spends its attention on the judgement calls below.

## Names carry the meaning

Constants, statics, enums, enum variants and struct fields carry no comments: the name says what the item is. `scripts/check-item-comments.py` enforces this in CI.

- Flag a name that needs its deleted comment to make sense. Rename it until it reads on its own.
- Give a primitive its own small type when its name cannot say its range or unit, such as a Track numbered 1 to 8.

## Glossary terms

`GLOSSARY.md` is the vocabulary for identifiers, messages, tests and docs.

- A defined term means exactly its definition. A different idea gets a different word: an undo entry is not a Step.
- An avoided word stays out of new code. PICO-8's and MIDI's own names are exempt where they mean the PICO-8 or MIDI thing, such as `music()` or a MIDI sequence.

## Renames are complete

One idea has one name. A rename covers every identifier, field, helper, test name, message and comment that uses the old word. Reread each touched comment as a sentence: a search-and-replace leaves phrases like "every cell in a cells".

## One home per fact

Each constant, format string and helper is defined once and imported where needed. Flag a second definition of the same value (a pitch limit, the SFX layout) or the same small logic (plural wording, a warning's place prefix).

## Tests

Tests exercise behaviour through a seam, in the domain's language: actions, Projects, Carts, warnings and samples. Refactoring without a behaviour change keeps every test green.

New tests use the highest seam that can express them:

1. **Headless App core**, the main seam: named actions and file contents in; the Project, the grid's text dump, the status line and Log, samples and written bytes out.
2. **Cart codec**: `.p8` and `.p8.png` bytes to and from Cart data.
3. **Music compilers**: a Cart or MIDI file to a Project and warnings, and a Project, Cart and Claim to a new Cart or failures.
4. **Offline synth**: Cart data or a Project to samples.
5. **Project file**: `.ptrk` text to and from a Project.

Expected data comes from real PICO-8 (saved Carts, measurements) or from a decision in the spec, never from the code under test. The spec decides that sound equality is proved through our synth. An imported Song renders the same samples as the original Cart, and an Exported Cart renders the same samples as the Project. A test that builds its expectation with `Voice` or `Player` for those comparisons follows the rule. A test that reads source text breaks it.
