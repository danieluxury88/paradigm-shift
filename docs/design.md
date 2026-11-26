# Design

Last verified: 2026-11-25

## Pitch

The same world looks different depending on whose experience you enter. In Paradigm Shift, one recurring traveler crosses distinct worlds and uses SHIFT to inhabit the perspectives of the people who live there. Each perspective reveals some paths and hides others; no single point of view explains the whole place. The title connects the jam theme to Thomas Kuhn's idea of a paradigm shift, while the wider story explores how people make sense of the same reality in different ways.

## Principles

- **Play first, name later.** Each level makes the player experience one idea through the mechanic; only after solving it does a concept card name the idea, with its origin, a real-world example and a "try it" habit.
- **Perspectives are situated.** A perspective reflects a person's experiences, needs and knowledge. It reveals real things while leaving other things out; it is never a simple good-side/bad-side label. Use fictional people, not stand-ins for real political parties, flags or public figures.
- **Keep the traveler, change the world.** The protagonist recurs across worlds. Each world brings new people, a new setting and its own visual language.
- **Data, not code.** Level maps and text live in `levels/*.ron`; the world manifest `levels/worlds.ron` sets the worlds, their order and what each level unlocks. `build.rs` embeds every level file, and [src/campaign.rs](../src/campaign.rs) validates the whole campaign at startup.
- **The game grows as you play.** Features such as lives, a timer or music are unlocked by levels in the manifest. Each feature is its own plugin gated with `run_if(unlocked(Feature::X))`, so it does not need to know which level turns it on (see [Progression](#progression)).

## Core loop

1. A level starts with a hint and two named perspectives.
2. The traveler moves, jumps and shifts between them. In the current prototype, SHIFT changes which tiles are solid; switching is refused if the other view has a wall where the player stands, as implemented in [src/lens.rs](../src/lens.rs).
3. Touching the goal opens the concept card.
4. ENTER loads the next level.

This is the playable foundation. The longer-term story makes each shift an encounter with a person's point of view: what they notice, what they fear or value, and what their surroundings mean to them. The player remains the traveler rather than becoming a different protagonist in each world.

## Progression

The experience is incremental: the first levels are only movement and SHIFT, and the game gains layers as the traveler advances. The schedule lives in `levels/worlds.ron` (`unlocks: [...]` per level), so it can be retuned without touching code.

| Layer | Where | What it holds |
|---|---|---|
| Content | `Campaign` in [src/campaign.rs](../src/campaign.rs) | Worlds and levels in play order, with what each level unlocks; read-only while playing |
| Player state | `Progress` in [src/campaign.rs](../src/campaign.rs) | The level being played; later the profile and records. Saved with the language by [src/save.rs](../src/save.rs) whenever it changes |
| Features | One plugin per feature | Systems gated with `run_if(unlocked(Feature::X))` |
| Card slot | `CardSlot` in [src/card.rs](../src/card.rs) | The same pattern on the concept card: features add a line, such as the level time |
| HUD slots | `Slot` in [src/hud.rs](../src/hud.rs) | The HUD owns the layout and spawns empty slots; each feature fills its own slot from an `Added<HudSlot>` query, which also covers the rebuild after a language change |

Current schedule:

| Level | Unlocks | Notes |
|---|---|---|
| 4 | Lives | Per-world budget (5, 4, 3 in the three worlds). Out of lives: restart the level in world 1, back to the world's first level in worlds 2 and 3 (`out_of_lives` in the manifest). Every respawn, fall or R, costs a life. |
| 5 | Asks the player's name (`ask: Name`) | Fills `{name}` in level texts from then on: the traveler becomes the player. Shown in the HUD's traveler slot. |
| 7 | Timer | A clock per level, shown in the HUD's timer slot; respawns do not stop it. Only reaching the goal (`LevelCompleted`) records a run, so the development skip key never sets a record. Best times are kept per level id in `Progress::best_times` and shown on the concept card through its `CardSlot`. |
| 10 | Asks the player's city (`ask: City`) | Fills `{city}`; used in the "Try it" habits of levels 10 and 12, which tie the concept to the player's own surroundings. |

Every screen that leads into a level (card, title, game over) goes through `Progress::level_entry`, which opens the question first when the level asks something the profile does not have yet.

## Worlds and levels

The game grows through worlds, each built around a broad philosophical question. A world provides the setting, its recurring people and its visual identity; its levels explore several related concepts through distinct situations.

| Layer | Purpose | What it contains |
|---|---|---|
| Game | Follow one traveler's changing understanding | The journey between worlds and the questions carried forward |
| World | Explore one broad question from several angles | A setting, a cast, a visual language and a sequence of related concepts |
| Level | Let the player experience one concept | A conflict, the perspectives involved, a mechanic and a small narrative change |

The current six-level plan can serve as the first world and teach the player how perspective shifting works. Its working arc is:

1. **Paradigm:** discover that the traveler can move through different models of the same place.
2. **Emic and etic:** move between an inhabitant's inside view and an observer's outside view.
3. **Straw man:** experience how a person's view can be distorted when someone else represents it unfairly.
4. **False dilemma:** find a path excluded by the way a problem was framed.
5. **Anomalies:** notice details that neither current perspective can explain comfortably.
6. **Synthesis:** combine what the traveler learned while preserving the differences between the people involved.

This arc introduces the premise; it is not the limit of the game's world count or philosophical scope. Later worlds can focus on other clusters already listed below, such as language and communication, ethics and power, or reality and identity. Those clusters are directions to explore, not a fixed release schedule.

### Next playable worlds

| World | Guiding question | Levels |
|---|---|---|
| The Listening City | What happens to meaning as it passes between people? | 7. Signal and noise; 8. The silent reply; 9. A fair reading |
| The Common Ground | How can a shared rule account for expertise, fairness and a person's concrete needs? | 10. The borrowed answer; 11. The unassigned seat; 12. The nearby need |

These worlds extend the same traveler's journey. In The Listening City, the traveler follows a warning through sender, listener and neighbor perspectives. In The Common Ground, the traveler moves between an expert's map, a rule maker's position and the needs of someone affected by the rule. The six new levels are playable-content drafts; their maps reuse the current lens and straw mechanics while their stories and concepts establish the larger world structure.

For every level, keep a small design brief: the question it raises, whose perspective the traveler enters, what becomes visible or possible, what remains outside that view, and what changes in the story after the puzzle. This keeps the concept, mechanic and character moment connected without turning levels into lectures.

## Concepts and how they could play

| Area | Concept | Mechanic | Status |
|---|---|---|---|
| Philosophy of science | Paradigm (Kuhn) | Ground and walls exist in only one perspective | Level 1 |
| Anthropology | Emic and etic (Pike, Harris) | Alternate between an insider's and an outsider's view to cross a gap | Level 2 |
| Philosophy of science | Anomaly and paradigm shifts | Notice details that do not fit either current perspective; the odd route suggests the map needs revisiting | Level 5 |
| Fallacies | Straw man | A bridge built from a distorted account of another person's view collapses; understanding what they mean reveals a route | Level 3 |
| Fallacies | False dilemma | Two obvious routes fail; another appears when the traveler questions the framing | Level 4 |
| Dialectics | Synthesis | The traveler carries useful parts of conflicting views into a route neither map showed alone | Level 6 |
| Communication | Signal and noise (Shannon) | Follow a warning through sender and listener perspectives | Level 7 |
| Communication | One cannot not communicate (Watzlawick et al.) | Read silence while checking how much of its meaning is inferred | Level 8 |
| Argumentation | Principle of charity | Restate a claim in its strongest reasonable form before challenging it | Level 9 |
| Epistemology | Appeal to authority | Weigh expertise alongside method, scope and local knowledge | Level 10 |
| Political philosophy | Veil of ignorance (Rawls) | Choose a rule before learning which position in the community will be yours | Level 11 |
| Ethics | Ethics of care (Gilligan and later care ethicists) | Consider relationships and concrete needs alongside shared rules | Level 12 |
| Useful tools | Steelmanning and Socratic questions | Unlockable abilities, e.g. "strengthen" a weak platform so it holds | Idea |
| Fallacies | Slippery slope | A literal slope that drags the player to the bottom unless they stop in time | Idea |
| Fallacies | Ad hominem | A route blocked by attacks on a person's character instead of their reasoning | Idea |
| Philosophy | Plato's cave | A shadow level where what you see is not what is there | Idea |

## Scope for the jam

Submission closes 2026-12-01 13:37 PST. The jam's opening-world target remains six levels (paradigm, emic/etic, straw man, false dilemma, anomalies, synthesis), a web build on itch.io, and a public GitHub repository. The Listening City and The Common Ground are expansion drafts for play and review; concepts marked "Idea" remain outside the jam target.

## Open questions

- Art direction: flat shapes and colour for the current prototype. For the larger game, each world should have a visual identity, and perspectives should change details, composition and meaning as well as colour.
- Sound: Bevy's `audio` feature is off to keep the web build small; add it only if there is time.
- Languages: English, German and Spanish are built in (L switches). The German and Spanish texts need a native review before the submission.
