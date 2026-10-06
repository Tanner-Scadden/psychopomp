# The shape of openness

A 4:24 deadpan design film, narrated in a cultivated British voice. The visual
language is flat Helvetica on near-black: exact OpenCode paths, geometric
draw-ons, image-plate pans, and a reconstructed cave hand stencil. Shapes
transform in place on the word that names them, and segments dip or dissolve
into one another. There are no orbs, particle forms, cards, or terminal chrome.

```sh
cargo run -p psychopomp-shape-of-openness
cargo run --release -- plan validate target/shape-of-openness/reel.json
cargo run --release -- plan render target/shape-of-openness/reel.json output/shape-of-openness/film-v2.mp4 --theme neutral
```

The Scene Program writes the reel and all thirteen independent segments under
`target/shape-of-openness/`. Choreography follows the actual narration words,
including ASR alternatives for British spelling and “Open Code”. The script,
word timings, duration manifest, and `stencil.json` are retained with the Scene
Program, so plans can be rebuilt without a voice or image API call. Generated
media lives under `output/shape-of-openness/`, and verification skips pixels
when it is absent. The first version is kept there as `film-v1.mp4`, with its
audio in `narration-v1/`.

## Voice

`narration/script.json` owns the directed text and settings: ElevenLabs
`eleven_v4`, the British generated voice **Severus Burbea**, stability 0.55,
similarity 0.75. Word timings come from the saved audio via Whisper, and the
transcripts contain no spoken direction tags. The tracked manifest keeps
durations and generation-cache hashes; request IDs are kept privately in
`output/shape-of-openness/generation-provenance-v2.json`.

To regenerate, copy `narration/script.json` to
`output/shape-of-openness/narration/script.json` and run
`bun --env-file=.env scripts/narrate.ts output/shape-of-openness/narration/script.json`.
Only clips whose text or settings changed are regenerated. Copy the resulting
manifest and word-timing files back beside the source script, omitting the
request IDs from the manifest, then rebuild and re-review the reel. `--draft`
uses local speech and needs no credentials.

## Pictures

`output/shape-of-openness/assets/` holds:

- `cave.webp` and `inferno.webp`, the two plates from the accompanying Notes
  essay (`public/mark/{cave,inferno}.webp`).
- `wall-gen.png` and `hand-gen.png`, two generated photographs: a bare,
  lamp-lit limestone wall with no marks, and the back of a spread right hand on
  chroma-key green.
- Layers derived from those two by `stencil.py`: the bare `wall.webp`, three
  breaths of ochre (`stencil-1..3.webp`, each an opaque plate), the keyed
  `hand.png`, its pigment-coated `hand-ochre.png`, `hand-shadow.png`, and
  `mist.png`. The spray is computed outward from the hand's own silhouette, so
  the stencil left behind is exactly the hand's absence. The script also writes
  `stencil.json`: layer boxes in the wall's 2048×1152 canvas, the outline the
  designer traces, and the absence as path data.
- `giotto/000..059.png` from `brush.py`, Giotto's circle as one red brush stroke
  with bristles that run dry, played as a 60 fps image sequence.

```sh
uv run --with numpy --with scipy --with pillow --with scikit-image python scenes/shape-of-openness/stencil.py
uv run --with numpy --with scipy --with pillow python scenes/shape-of-openness/brush.py
```

## Source and fiction

This is a contemporary formal inquiry, not an account of the original brand's
design process. The proposals, the rationale, and the personnel notes are
fictional. The image plates and the hand reconstruction are generated
contemporary images, not historical evidence. The film says so in its plate
labels and end credits.

- [Official OpenCode artwork](https://opencode.ai/brand): outer 240×300, perimeter
  60, counter 120×180, secondary plane 120×120. The icon paths are translated only
  to center them in their view box. Fourteen of twenty modules form the perimeter
  and six form the counter; four of those carry the secondary plane. By area the
  mark is 70% perimeter, 20% plane, and 10% open counter.
- [Chauvet](https://archeologie.culture.gouv.fr/chauvet/en/datings) and
  [Lascaux techniques](https://archeologie.culture.gouv.fr/lascaux/en/techniques):
  context for early image-making and blown-pigment stencils; the film makes no
  claim that a prehistoric mark depicted OpenCode.
- [Euclid, Book I, Definition 15](https://mathcs.clarku.edu/~djoyce/elements/bookI/defI15.html)
  and [Proposition 1](https://mathcs.clarku.edu/~djoyce/elements/bookI/propI1.html):
  constant radius and the equal-circle construction behind the vesica and the
  equilateral pointed arch.
- Vasari's *Life of Giotto*: the freehand red circle sent to the Pope is
  attributed to Vasari, not presented as independently verified history.
- Dante's *Inferno*: nine circles narrowing to a frozen centre. The concentric
  diagram is a schematic; the imagined plate is atmospheric, not a literal map.
- The golden ratio is approximately 1.618; the mark's actual height/width is 1.25.
  Rejected specimens are drawn lookalikes, not source artwork for other brands.

## Reusable library support

- `Face::Sans` and `SansBold`: installed Helvetica Neue, using the existing font path.
- `StagePost::FLAT`: no bloom, grain, vignette, or backdrop.
- Editorial-scale type (up to 320 px), artwork (up to 2048 px), and shape strokes
  (up to 256 px); connector bounds stay unchanged.
- Optional explicit sRGB icon `ink`, for artwork that must retain its pigment
  under a neutral theme.

The film uses existing Stage shapes, paths, SVG coverage, footage (straight-alpha
stills registered at one depth, which keep declaration order, and an image
sequence), camera channels, and word-timed narration. It adds no scene graph or
second renderer.

## Delivery verification

- Final MP4: 1920×1080 H.264, 60 fps, 15,861 frames, 264.35 seconds, mono AAC at
  48 kHz. Both streams start at zero and end together. The film rendered in one
  pass in 17 minutes.
- The audio received a final loudness pass, +2.4 dB into a 4× oversampled limiter
  at 0.86, with the video stream copied unchanged. Measured: −16.1 LUFS
  integrated, −1.2 dBTP, 4.6 LU loudness range.
- Sync: Whisper on the delivered soundtrack matched 264 of 266 distinctive words
  to their plan times with a 5 ms median and 29 ms at the 95th percentile. The
  larger start differences fall on words after a pause, whose ends agree to within
  about 30 ms.
- Contact sheets cover all thirteen sections. Encoded motion samples were
  inspected at 80 ms spacing for the hand's approach and withdrawal, the opening
  and closing camera flights, and the descent through the circles.
- Workspace tests, formatting, and Clippy pass. `verify compare` against the
  previous film's baseline reports every other scene's plans and frames unchanged.
