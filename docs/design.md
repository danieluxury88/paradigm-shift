# Design

Last verified: 2026-11-21

## Pitch

The same world looks different depending on the frame you see it through. In Paradigm Shift, SHIFT changes the lens and the level transforms: what one worldview treats as a wall, another treats as a bridge. No single lens reaches the goal. The title is both the jam theme and Thomas Kuhn's idea of a paradigm shift, which is the thread of the whole game.

## Principles

- **Play first, name later.** Each level makes the player experience one idea through the mechanic; only after solving it does a concept card name the idea, with its origin, a real-world example and a "try it" habit.
- **No side is the right one.** Lenses use abstract axes (order/freedom, tradition/change, collective/individual, insider/outsider). No real parties, flags or people. The point is that every view has blind spots, not that one view wins.
- **Data, not code.** Levels and concept cards live in `levels/*.ron`, so new concepts can be added without touching Rust.

## Core loop

1. A level starts with a hint and two named lenses.
2. The player moves, jumps and switches lens. Switching is refused if the other view has a wall where the player stands.
3. Touching the goal opens the concept card.
4. ENTER loads the next level.

## Concepts and how they could play

| Area | Concept | Mechanic | Status |
|---|---|---|---|
| Philosophy of science | Paradigm (Kuhn) | Ground and walls exist in only one lens | Level 1 |
| Anthropology | Emic and etic (Pike, Harris) | Alternating steps over a pit; switch in mid-air | Level 2 |
| Philosophy of science | Anomalies and paradigm shifts | Objects that do not fit the current lens flicker; collecting enough unlocks a new lens | Planned |
| Fallacies | Straw man | A straw bridge that looks like ground in the "win the debate" lens and collapses when stood on; the real crossing appears in "understand the other" | Level 3 |
| Fallacies | False dilemma | Two obvious roads (climb, or walk on) both end at a wall; a third, middle road appears in "look for more" | Level 4 |
| Fallacies | Slippery slope | A literal slope that drags the player to the bottom unless they stop in time | Idea |
| Fallacies | Ad hominem, appeal to authority | Characters who block the way with personal attacks or "because I say so" | Idea |
| Philosophy | Plato's cave | A shadow level where what you see is not what is there | Idea |
| Useful tools | Steelmanning, principle of charity, Socratic questions | Unlockable abilities, e.g. "strengthen" a weak platform so it holds | Idea |
| Communication | Noise (Shannon), "one cannot not communicate" (Watzlawick) | Characters' messages distort depending on the lens you hear them through | Idea |
| Dialectics | Thesis, antithesis, synthesis | Final level: the two lenses merge into a third, and two characters with opposite views must cooperate | Planned (final) |

## Scope for the jam

Submission closes 2026-12-01 13:37 PST. Target: six levels (lenses, emic/etic, straw man, false dilemma, anomalies, synthesis), a web build on itch.io, and a public GitHub repository. Anything marked "Idea" waits until after the jam.

## Open questions

- Art direction: flat shapes and colour for now; a consistent minimal style may be enough for the jam.
- Sound: Bevy's `audio` feature is off to keep the web build small; add it only if there is time.
- Languages: English, German and Spanish are built in (L switches). The German and Spanish texts need a native review before the submission.
