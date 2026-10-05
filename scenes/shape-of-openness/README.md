# The shape of openness

A 3:20 deadpan design film, narrated in a cultivated British voice. The visual
language is flat Helvetica on near-black: exact OpenCode paths, geometric
draw-ons, image-plate pans, measured close-ups, and hard cuts. There are no orbs,
particle forms, cards, or terminal chrome.

```sh
cargo run -p psychopomp-shape-of-openness
cargo run --release -- plan validate target/shape-of-openness/reel.json
cargo run --release -- plan render target/shape-of-openness/reel.json output/shape-of-openness/film.mp4 --theme neutral
```

The Scene Program writes the reel and all thirteen independent segments under
`target/shape-of-openness/`. Choreography follows the actual narration words,
including ASR alternatives for British spelling and “Open Code”.
Generated audio lives under `output/shape-of-openness/narration/`; the two generated
WebP plates live under `output/shape-of-openness/assets/`. The script, timings,
and duration manifest are retained with the Scene Program, so plans can be rebuilt
without a voice API call. Verification skips pixels when these ignored inputs are
absent.

## Voice

`narration/script.json` owns the directed text and settings. ElevenLabs
`eleven_v4`, the British generated voice **Severus Burbea**, stability 0.55,
similarity 0.75. The opening was generated and played as an audition before the
remaining twelve clips. Word timings come from the saved audio via Whisper.
The transcript contains no spoken direction tags. The narration manifest retains
durations and generation-cache hashes; request IDs are retained privately under
`output/shape-of-openness/`.

To regenerate, copy `narration/script.json` to
`output/shape-of-openness/narration/script.json`, then run `bun scripts/narrate.ts`
on that copy with `ELEVENLABS_API_KEY` injected. Copy the resulting manifest and
word-timing JSON files back beside the source script, omitting private request IDs
from the manifest. Rebuild and re-review the reel. `--draft` uses local speech and
needs no credentials. The accompanying Notes essay's `public/mark/{cave,inferno}.webp`
files supply the plates.

## Source and fiction

This is a contemporary formal inquiry, not an account of the original brand's
design process. The mock proposals and personnel notes are fictional. Both image
plates are generated contemporary illustrations made for the accompanying essay,
not historical evidence. The film says so in its plate labels and end credits.

- [Official OpenCode artwork](https://opencode.ai/brand): outer 240×300, perimeter
  60, counter 120×180, secondary plane 120×120. The icon paths are translated only
  to center them in their view box. Fourteen of twenty modules form the perimeter;
  six form the counter; four of those carry the secondary plane.
- [Chauvet](https://archeologie.culture.gouv.fr/chauvet/en/datings) and
  [Lascaux techniques](https://archeologie.culture.gouv.fr/lascaux/en/techniques):
  context for early image-making and stencil operations; the film makes no claim
  that a prehistoric mark depicted OpenCode.
- [Euclid, Book I, Definition 15](https://mathcs.clarku.edu/~djoyce/elements/bookI/defI15.html)
  and [Proposition 1](https://mathcs.clarku.edu/~djoyce/elements/bookI/propI1.html):
  constant radius and equal-circle construction. Sacred meaning is interpretation.
- Vasari's *Life of Giotto*: the hand-drawn-circle anecdote is explicitly attributed
  to Vasari, rather than presented as independently verified history.
- Dante's *Inferno*: nine circles, with ice in the ninth. The concentric diagram
  is a schematic; the imagined plate is atmospheric, not a literal architectural map.
- The golden ratio is approximately 1.618; the mark's actual height/width is 1.25.
  Rejected specimens are drawn lookalikes, not source artwork for other brands.

## Reusable library support

- `Face::Sans` and `SansBold`: installed Helvetica Neue, using the existing font path.
- `StagePost::FLAT`: no bloom, grain, vignette, or backdrop.
- Editorial-scale type (up to 320 px), artwork (up to 2048 px), and shape strokes
  (up to 256 px); connector bounds stay unchanged.
- Optional explicit sRGB icon `ink`, for artwork that must retain its pigment
  under a neutral theme. Existing plans omit it and preserve their prior pixels.

The film uses existing Stage shapes, SVG coverage, footage, camera channels, and
word-timed narration. It adds no scene graph or second renderer.

## Delivery verification

- Final MP4: 1920×1080 H.264, 60 fps, 11,976 frames, 199.6 seconds, mono AAC at 48 kHz.
- Measured final audio: −16.41 LUFS integrated, −1.40 dBTP, 4.4 LU loudness range.
- Contact sheets cover all thirteen sections; the counter pullback was inspected
  in an encoded motion sample. The final encoded join was checked frame-by-frame.
- A command timeout stopped the first render after frame 6,619. The remaining
  5,356 frames were rendered starting at 110⅓ seconds, copied after the first
  6,620 frames, and remuxed with continuous source narration on the plan clock.
  The audio received a final loudness pass. Both streams begin at zero.
- Workspace tests, formatting, Clippy, and the explicit-pigment GPU test pass.
  All 252 existing reference frames retain their pixels; only this scene is new.
