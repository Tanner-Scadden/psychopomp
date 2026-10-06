//! A flat editorial film. The narration supplies the clock; exact artwork,
//! geometric studies, image plates, and a reconstructed hand stencil supply
//! the pictures. Each beat moves on the word that names it, and things
//! transform in place rather than cut.
use anyhow::{Context, Result};
use psychopomp::{
    author::{PlanBuilder, seconds},
    caption::CaptionAlign,
    face::Face,
    footage::{self, Clip, Fit, Mask},
    math::easing::Ease,
    narration::{Narration, Spoken},
    plan::{ReelPlan, ReelSegmentPlan},
    stage::{
        Arrow, Curve, Figure, Fill, Material, StageActor, StageElement, StagePlan, StagePost,
        Waypoint,
    },
    tone::Tone,
};
use std::{f32::consts::TAU, fs, path::PathBuf};

// Official paths translated +30 on x to center their 240×300 artwork in a 300-square view.
const RING: &str = "M210 60H90V240H210V60ZM270 300H30V0H270V300Z";
const PLANE: &str = "M210 240H90V120H210V240Z";
const DISC: &str = "M150 0A150 150 0 0 1 150 300A150 150 0 0 1 150 0Z";
const ANNULUS: &str = "M150 0A150 150 0 0 1 150 300A150 150 0 0 1 150 0ZM150 62A88 88 0 0 0 150 238A88 88 0 0 0 150 62Z";
const REFRESHMENT_RED: &str = "M42 121A112 112 0 0 1 261 139C206 107 129 188 42 121Z";
const REFRESHMENT_BLUE: &str = "M39 166C128 234 205 132 261 167A112 112 0 0 1 39 166Z";
const OPERA: &str = "M150 20A95 130 0 1 1 149.99 20ZM150 65A50 85 0 1 0 150.01 65Z";
const GOLDEN: &str = "M103.64745 60V240H196.35255V60ZM57.2949 0H242.7051V300H57.2949Z";
const BLACK: [u8; 3] = [8, 8, 7];
const DRAW: Ease = Ease::CubicBezier([0.45, 0.0, 0.2, 1.0]);
const COMPASS: Ease = Ease::CubicBezier([0.42, 0.0, 0.32, 1.0]);
/// Accelerating away, for exits and falls.
const AWAY: Ease = Ease::CubicBezier([0.55, 0.0, 0.9, 0.45]);
/// Accelerating into a doorway: the camera flies through an opening.
const THROUGH: Ease = Ease::CubicBezier([0.55, 0.0, 0.85, 0.3]);

enum Enter {
    Cut,
    Dip(f64),
    Dissolve(f64),
}

/// Each segment's silence before and after its narration, and how it enters.
const PARTS: [(&str, f64, f64, Enter); 13] = [
    ("opening", 0.5, 2.4, Enter::Cut),
    ("cave", 0.7, 1.8, Enter::Dip(1.0)),
    ("circle", 0.6, 1.7, Enter::Dip(0.9)),
    ("vesica", 0.4, 1.8, Enter::Dissolve(0.7)),
    ("ratio", 0.4, 1.8, Enter::Dissolve(0.8)),
    ("gesture", 0.6, 1.9, Enter::Dip(0.9)),
    ("inferno", 0.6, 1.6, Enter::Dissolve(1.0)),
    ("grid", 0.6, 1.1, Enter::Dip(0.9)),
    ("counter", 0.4, 1.7, Enter::Dissolve(0.6)),
    ("optics", 0.5, 1.8, Enter::Dip(0.8)),
    ("pepsi", 0.6, 2.0, Enter::Dip(0.9)),
    ("alternatives", 0.6, 1.9, Enter::Dip(0.9)),
    ("finale", 0.6, 7.2, Enter::Dip(1.0)),
];

fn s(value: f64) -> u64 {
    seconds(value)
}
fn before(at: u64, value: f64) -> u64 {
    at.saturating_sub(s(value))
}
/// Seconds from `from` to `to`.
fn span(from: u64, to: u64) -> f32 {
    to.saturating_sub(from) as f32 / 1e9
}
/// The fraction of an ease's duration at which it reaches `progress`.
fn reaches(curve: Ease, progress: f32) -> f32 {
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..40 {
        let middle = (low + high) * 0.5;
        if curve.sample(middle) < progress {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) * 0.5
}

fn label(id: &str, at: [f32; 3], size: f32, value: &str, tone: Tone, face: Face) -> StageElement {
    StageElement::label(id, at, size, &[(value, tone)]).face(face)
}
fn text(id: &str, x: f32, y: f32, size: f32, value: &str) -> StageElement {
    label(id, [x, y, -1.0], size, value, Tone::Plain, Face::Sans)
}
fn note(id: &str, x: f32, y: f32, value: &str) -> StageElement {
    label(id, [x, y, -1.0], 25.0, value, Tone::Muted, Face::Sans)
}
fn small(id: &str, x: f32, y: f32, size: f32, value: &str) -> StageElement {
    label(id, [x, y, -1.0], size, value, Tone::Muted, Face::Sans)
}
/// A provenance note over a photographic plate, on its [`scrim`].
fn plate_note(id: &str, value: &str) -> StageElement {
    label(
        id,
        [960.0, 1040.0, -1.0],
        22.0,
        value,
        Tone::Plain,
        Face::Sans,
    )
}
/// A soft dark band along the bottom edge that keeps plate notes legible;
/// its `blur` channel feathers it.
fn scrim(id: &str) -> StageElement {
    let mut band = shape(
        id,
        [960.0, 1085.0],
        Figure::Rect([2300.0, 190.0]),
        Some(Fill::Material(Material::Background)),
        None,
        0.25,
    );
    if let StageElement::Shape { fill_opacity, .. } = &mut band {
        *fill_opacity = 0.82;
    }
    at_z(band, -0.5)
}
fn icon(id: &str, at: [f32; 2], size: f32, path: &str, tone: Tone) -> StageElement {
    StageElement::Icon {
        id: id.into(),
        at: [at[0], at[1], 0.0],
        size,
        icon: String::new(),
        path: path.into(),
        view: 300.0,
        ink: None,
        tone,
    }
}
/// Artwork whose color is part of its identity.
fn art(id: &str, at: [f32; 2], size: f32, path: &str, ink: [u8; 3]) -> StageElement {
    let mut element = icon(id, at, size, path, Tone::Plain);
    if let StageElement::Icon { ink: own, .. } = &mut element {
        *own = Some(ink);
    }
    element
}
fn shape(
    id: &str,
    at: [f32; 2],
    figure: Figure,
    fill: Option<Fill>,
    stroke: Option<Tone>,
    width: f32,
) -> StageElement {
    StageElement::Shape {
        id: id.into(),
        at: [at[0], at[1], 0.0],
        shape: figure,
        corner: 0.0,
        fill,
        fill_opacity: 1.0,
        stroke,
        width: width.max(0.25),
        dash: None,
        arrow: Arrow::None,
    }
}
fn outline(id: &str, at: [f32; 2], figure: Figure, width: f32) -> StageElement {
    shape(id, at, figure, None, Some(Tone::Plain), width)
}
fn solid(id: &str, at: [f32; 2], figure: Figure, tone: Tone) -> StageElement {
    shape(id, at, figure, Some(Fill::Tone(tone)), None, 0.25)
}
/// A crisp line from `from` to `to`: a sliver polygon whose stroke draws on
/// out and back, so drawing it to one half draws the line.
fn rule(id: &str, from: [f32; 2], to: [f32; 2], width: f32, tone: Tone) -> StageElement {
    let [dx, dy] = [to[0] - from[0], to[1] - from[1]];
    let length = dx.hypot(dy).max(1e-3);
    let side = [-dy / length * 0.01, dx / length * 0.01];
    shape(
        id,
        from,
        Figure::Polygon(vec![[0.0, 0.0], [dx, dy], [dx + side[0], dy + side[1]]]),
        None,
        Some(tone),
        width,
    )
}
/// A matte construction line, optionally with arrowheads for a dimension.
fn guide(id: &str, points: &[[f32; 2]], arrow: Arrow) -> StageElement {
    StageElement::Path {
        id: id.into(),
        through: points
            .iter()
            .map(|p| Waypoint::Point([p[0], p[1], 0.0]))
            .collect(),
        curve: Curve::Straight,
        corner: 0.0,
        bend: 0.0,
        tone: Tone::Plain,
        width: 1.2,
        dash: None,
        arrow,
    }
}
/// A radius with a small diamond at its tip, pointing up from its center.
fn compass_arm(id: &str, at: [f32; 2], length: f32) -> StageElement {
    let tip = length;
    solid(
        id,
        at,
        Figure::Polygon(vec![
            [-1.1, 0.0],
            [-1.1, -tip + 7.0],
            [-6.5, -tip],
            [0.0, -tip - 6.5],
            [6.5, -tip],
            [1.1, -tip + 7.0],
            [1.1, 0.0],
        ]),
        Tone::Plain,
    )
}
fn plate(id: &str, at: [f32; 3], size: [f32; 2], fit: Fit) -> StageElement {
    StageElement::Footage {
        id: id.into(),
        at,
        size,
        clip: Clip::new(id),
        fit,
        mask: Mask::default(),
        framed: false,
        tint: Tone::Plain,
    }
}
fn at_z(mut element: StageElement, z: f32) -> StageElement {
    match &mut element {
        StageElement::Shape { at, .. }
        | StageElement::Icon { at, .. }
        | StageElement::Label { at, .. }
        | StageElement::Footage { at, .. } => at[2] = z,
        _ => {}
    }
    element
}
fn rounded(mut element: StageElement, radius: f32) -> StageElement {
    match &mut element {
        StageElement::Shape { corner, .. } => *corner = radius,
        StageElement::Footage { mask, .. } => *mask = Mask::Rect { radius },
        _ => {}
    }
    element
}
fn dashed(mut element: StageElement, pattern: [f32; 2]) -> StageElement {
    if let StageElement::Shape { dash, .. } = &mut element {
        *dash = Some(pattern);
    }
    element
}
fn aligned(element: StageElement, align: CaptionAlign) -> StageElement {
    element.align(align)
}
fn still(sc: &mut PlanBuilder, id: &str, file: &str) {
    let end = sc.duration_nanos();
    sc.media(footage::still(id, format!("assets/{file}"), 0, end));
}

/// One segment's Stage and the plan it writes into. Every element starts
/// hidden, and appears when its beat says so.
struct Shot<'a> {
    stage: StageActor,
    sc: &'a mut PlanBuilder,
}

impl<'a> Shot<'a> {
    fn new(sc: &'a mut PlanBuilder, elements: Vec<StageElement>) -> Result<Self> {
        let plan = StagePlan {
            post: StagePost::FLAT,
            elements,
        };
        let mut stage = StageActor::declare(sc, "stage", &plan)?;
        for element in &plan.elements {
            stage.channel(sc, &format!("{}.opacity", element.id()), 0.0);
        }
        Ok(Self { stage, sc })
    }
    fn end(&self) -> u64 {
        self.sc.duration_nanos()
    }
    fn anchor(&self, id: &str) -> [f32; 2] {
        let at = self.stage.plan().element(id).and_then(|e| e.anchor());
        let at = at.unwrap_or_else(|| panic!("'{id}' has no position"));
        [at[0], at[1]]
    }
    fn init(&mut self, property: &str, value: f32) {
        self.stage.channel(self.sc, property, value);
    }
    fn set(&mut self, property: &str, at: u64, value: f32) {
        self.stage.set(self.sc, property, at, value);
    }
    fn ease(&mut self, property: &str, at: u64, target: f32, seconds: f32, curve: Ease) {
        self.stage
            .ease(self.sc, property, at, target, seconds, curve);
    }
    fn glide(&mut self, property: &str, at: u64, target: f32, seconds: f32) {
        self.stage.glide(self.sc, property, at, target, seconds);
    }
    fn spring(&mut self, property: &str, at: u64, target: f32, seconds: f32) {
        self.stage.to(self.sc, property, at, target, seconds);
    }
    fn bounce(&mut self, property: &str, at: u64, target: f32, seconds: f32, bounce: f32) {
        self.stage
            .bounce(self.sc, property, at, target, seconds, bounce);
    }
    /// Jump to `from`, then ease to `to`.
    fn tween(&mut self, property: &str, at: u64, [from, to]: [f32; 2], seconds: f32, curve: Ease) {
        self.set(property, at, from);
        self.ease(property, at, to, seconds, curve);
    }
    fn hit(&mut self, property: &str, at: u64, peak: f32) {
        self.stage.hit(self.sc, property, at, peak, 0.0);
    }
    fn jolt(&mut self, at: u64, direction: [f32; 2], strength: f32) {
        self.stage.jolt(self.sc, at, direction, strength);
    }
    fn show(&mut self, ids: &[&str], at: u64) {
        for id in ids {
            self.set(&format!("{id}.opacity"), at, 1.0);
        }
    }
    fn hide(&mut self, ids: &[&str], at: u64) {
        for id in ids {
            self.set(&format!("{id}.opacity"), at, 0.0);
        }
    }
    fn fade(&mut self, id: &str, at: u64, opacity: f32, seconds: f32) {
        self.ease(
            &format!("{id}.opacity"),
            at,
            opacity,
            seconds,
            Ease::Smootherstep,
        );
    }
    fn appear(&mut self, id: &str, at: u64, seconds: f32) {
        self.fade(id, at, 1.0, seconds);
    }
    fn vanish(&mut self, id: &str, at: u64, seconds: f32) {
        self.fade(id, at, 0.0, seconds);
    }
    /// Type rises a little into place as it fades in.
    fn rise(&mut self, id: &str, at: u64) {
        self.set(&format!("{id}.y"), at, 22.0);
        self.spring(&format!("{id}.y"), at, 0.0, 0.9);
        self.appear(id, at, 0.55);
    }
    /// A rigid panel drifts into place with a small scale correction.
    fn settle(&mut self, id: &str, at: u64) {
        self.set(&format!("{id}.scale"), at, 1.05);
        self.bounce(&format!("{id}.scale"), at, 1.0, 0.65, 0.12);
        self.set(&format!("{id}.y"), at, 16.0);
        self.bounce(&format!("{id}.y"), at, 0.0, 0.6, 0.14);
        self.appear(id, at, 0.25);
    }
    /// A dot arrives with a small overshoot.
    fn pop(&mut self, id: &str, at: u64) {
        self.set(&format!("{id}.scale"), at, 2.2);
        self.bounce(&format!("{id}.scale"), at, 1.0, 0.5, 0.25);
        self.appear(id, at, 0.15);
    }
    fn draw_to(&mut self, id: &str, at: u64, target: f32, seconds: f32) {
        self.set(&format!("{id}.opacity"), at, 1.0);
        self.tween(&format!("{id}.draw"), at, [0.0, target], seconds, DRAW);
    }
    fn draw(&mut self, id: &str, at: u64, seconds: f32) {
        self.draw_to(id, at, 1.0, seconds);
    }
    /// A [`rule`] draws to the end of its first side.
    fn line(&mut self, id: &str, at: u64, seconds: f32) {
        self.draw_to(id, at, 0.5, seconds);
    }
    fn shift(&mut self, id: &str, at: u64, [x, y]: [f32; 2], seconds: f32, curve: Ease) {
        self.ease(&format!("{id}.x"), at, x, seconds, curve);
        self.ease(&format!("{id}.y"), at, y, seconds, curve);
    }
    /// Arrive from an offset, settling exactly into place.
    fn arrive(&mut self, id: &str, at: u64, [x, y]: [f32; 2], seconds: f32, curve: Ease) {
        self.set(&format!("{id}.x"), at, x);
        self.set(&format!("{id}.y"), at, y);
        self.shift(id, at, [0.0, 0.0], seconds, curve);
    }
    /// Move elements as one rigid group about `pivot`: scaled, then shifted.
    fn regroup(
        &mut self,
        ids: &[&str],
        pivot: [f32; 2],
        scale: f32,
        shift: [f32; 2],
        at: u64,
        seconds: f32,
    ) {
        for id in ids {
            let [x, y] = self.anchor(id);
            let to = [
                (x - pivot[0]) * (scale - 1.0) + shift[0],
                (y - pivot[1]) * (scale - 1.0) + shift[1],
            ];
            self.shift(id, at, to, seconds, Ease::Smootherstep);
            self.glide(&format!("{id}.scale"), at, scale, seconds);
        }
    }
    /// Sweep a circle on from twelve o'clock while its radius turns with the pen.
    fn compass(&mut self, circle: &str, arm: &str, at: u64, seconds: f32) {
        self.set(&format!("{circle}.opacity"), at, 1.0);
        self.tween(&format!("{circle}.draw"), at, [0.0, 1.0], seconds, COMPASS);
        self.ease(&format!("{arm}.rotation"), at, TAU, seconds, COMPASS);
    }
    fn type_in(&mut self, label: &str, at: u64, per_second: f32) {
        self.stage.type_in(self.sc, label, at, per_second);
    }
    /// The radius grows out of the center.
    fn extend(&mut self, arm: &str, at: u64) {
        self.appear(arm, at, 0.12);
        self.set(&format!("{arm}.scale"), at, 0.02);
        self.spring(&format!("{arm}.scale"), at, 1.0, 0.55);
    }
    fn retract(&mut self, arm: &str, at: u64) {
        self.spring(&format!("{arm}.scale"), at, 0.02, 0.45);
        self.vanish(arm, at + s(0.2), 0.25);
    }
}

fn opening(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let c = [960.0, 540.0];
    let size = 620.0;
    let u = size / 300.0;
    let [left, right] = [c[0] - 120.0 * u, c[0] + 120.0 * u];
    let [top, bottom] = [c[1] - 150.0 * u, c[1] + 150.0 * u];
    let mut k = Shot::new(
        sc,
        vec![
            outline("hairline", c, Figure::Rect([120.0 * u, 180.0 * u]), 1.5),
            icon("ring", c, size, RING, Tone::Plain),
            guide("guide-left", &[[left, 40.0], [left, 1040.0]], Arrow::None),
            guide("guide-top", &[[80.0, top], [1840.0, top]], Arrow::None),
            guide(
                "guide-right",
                &[[right, 40.0], [right, 1040.0]],
                Arrow::None,
            ),
            guide(
                "guide-bottom",
                &[[80.0, bottom], [1840.0, bottom]],
                Arrow::None,
            ),
            small("width", c[0], top - 30.0, 24.0, "240"),
            small("height", right + 46.0, c[1], 24.0, "300"),
            text("title-1", 960.0, 470.0, 126.0, "The shape"),
            text("title-2", 960.0, 612.0, 126.0, "of openness."),
            note("byline", 960.0, 800.0, "An inquiry into the OpenCode mark"),
        ],
    )?;
    let guides = ["guide-left", "guide-top", "guide-right", "guide-bottom"];
    // An opening is drawn first, then the walls close in around it.
    k.draw("hairline", v.at("An opening"), 1.6);
    let walls = before(v.at("surrounds"), 0.12);
    k.init("camera.zoom", 1.16);
    k.glide("camera.zoom", 0, 1.0, span(0, walls + s(0.4)));
    k.appear("ring", walls, 0.45);
    k.init("ring.scale", 1.5);
    k.spring("ring.scale", walls, 1.0, 1.15);
    k.init("ring.blur", 16.0);
    k.ease("ring.blur", walls, 0.0, 0.8, Ease::Smootherstep);
    k.vanish("hairline", walls + s(0.9), 0.5);
    k.glide("camera.zoom", v.at("We have"), 1.06, 5.6);
    let careful = v.at("very carefully");
    for (i, id) in guides.iter().enumerate() {
        k.draw(id, careful + s(0.12 * i as f64), 1.0);
    }
    let expense = v.at("considerable");
    k.rise("width", expense);
    k.rise("height", expense + s(0.12));
    // Then the camera flies through the opening, and the title is inside.
    let this = v.at("This is");
    for id in guides.iter().chain(&["width", "height"]) {
        k.vanish(id, before(this, 0.2), 0.4);
    }
    k.ease("camera.zoom", this + s(0.05), 16.0, 1.15, THROUGH);
    let through = this + s(1.2);
    k.set("camera.zoom", through, 1.0);
    k.hide(&["ring"], through);
    k.rise("title-1", v.at("shape").max(through));
    k.rise(
        "title-2",
        before(v.at("openness"), 0.05).max(through + s(0.15)),
    );
    k.appear("byline", v.at("openness") + s(1.0), 0.8);
    Ok(())
}

/// The cave close-up's layers, derived by `stencil.py` from the hand that
/// made the stencil, so its absence registers exactly.
struct Stencil {
    layers: Vec<(String, [f32; 4])>,
    outline: Vec<[f32; 2]>,
    absence: ([f32; 3], String),
}

impl Stencil {
    fn load() -> Result<Self> {
        let json: serde_json::Value = serde_json::from_str(include_str!("../stencil.json"))?;
        let numbers = |value: &serde_json::Value| -> Vec<f32> {
            value
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|n| n.as_f64())
                        .map(|n| n as f32)
                        .collect()
                })
                .unwrap_or_default()
        };
        let layers = json["layers"]
            .as_object()
            .context("stencil layers")?
            .iter()
            .map(|(id, value)| {
                let b = numbers(value);
                (id.clone(), [b[0], b[1], b[2], b[3]])
            })
            .collect();
        let outline = json["outline"]
            .as_array()
            .context("stencil outline")?
            .iter()
            .map(|point| {
                let p = numbers(point);
                [p[0], p[1]]
            })
            .collect();
        let frame = numbers(&json["absence"]["box"]);
        let path = json["absence"]["path"]
            .as_str()
            .context("absence path")?
            .to_owned();
        Ok(Self {
            layers,
            outline,
            absence: ([frame[0], frame[1], frame[2]], path),
        })
    }
    fn layer(&self, id: &str) -> [f32; 4] {
        self.layers
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, b)| *b)
            .unwrap_or_else(|| panic!("stencil.json has no layer '{id}'"))
    }
    /// The share of the outline's perimeter before its closing edge, which
    /// runs along the bottom of the canvas.
    fn traced(&self) -> f32 {
        let edge = |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
        let n = self.outline.len();
        let open: f32 = (1..n)
            .map(|i| edge(self.outline[i - 1], self.outline[i]))
            .sum();
        open / (open + edge(self.outline[n - 1], self.outline[0]))
    }
}

/// The wall's 2048×1152 canvas sits 1:1 in world pixels, centered.
fn canvas([x, y]: [f32; 2]) -> [f32; 2] {
    [x - 1024.0 + 960.0, y - 576.0 + 540.0]
}
const WALL_Z: f32 = 10.0;
fn layer(id: &str, b: [f32; 4]) -> StageElement {
    let at = canvas([b[0] + b[2] * 0.5, b[1] + b[3] * 0.5]);
    plate(id, [at[0], at[1], WALL_Z], [b[2], b[3]], Fit::Fill)
}

fn cave(sc: &mut PlanBuilder, v: &Spoken<'_>, stencil: &Stencil) -> Result<()> {
    let hand = stencil.layer("hand");
    let trace_at = canvas([hand[0] + hand[2] * 0.5, hand[1] + hand[3] * 0.5]);
    let trace = stencil
        .outline
        .iter()
        .map(|&p| {
            let p = canvas(p);
            [p[0] - trace_at[0], p[1] - trace_at[1]]
        })
        .collect();
    let (frame, path) = &stencil.absence;
    let absence_at = canvas([frame[0] + frame[2] * 0.5, frame[1] + frame[2] * 0.5]);
    let page = [2480.0, 450.0];
    let mut elements = vec![
        plate("cave", [960.0, 540.0, 12.0], [1920.0, 1080.0], Fit::Cover),
        plate("wall", [960.0, 540.0, WALL_Z], [2048.0, 1152.0], Fit::Fill),
    ];
    for id in [
        "stencil-1",
        "stencil-2",
        "stencil-3",
        "hand-shadow",
        "hand",
        "hand-ochre",
        "mist",
    ] {
        elements.push(layer(id, stencil.layer(id)));
    }
    elements.extend([
        at_z(
            rounded(outline("trace", trace_at, Figure::Polygon(trace), 2.4), 5.0),
            WALL_Z,
        ),
        at_z(
            StageElement::Icon {
                id: "absence".into(),
                at: [absence_at[0], absence_at[1], 0.0],
                size: frame[2],
                icon: String::new(),
                path: path.clone(),
                view: frame[2],
                ink: Some(BLACK),
                tone: Tone::Plain,
            },
            WALL_Z,
        ),
        scrim("scrim"),
        plate_note(
            "plate-note",
            "Imagined plate · after the hand stencils of Chauvet",
        ),
        plate_note("reconstruction", "Imagined reconstruction"),
        outline(
            "page",
            page,
            Figure::Polygon(vec![
                [-200.0, -260.0],
                [130.0, -260.0],
                [200.0, -190.0],
                [200.0, 260.0],
                [-200.0, 260.0],
            ]),
            4.0,
        ),
        outline(
            "fold",
            page,
            Figure::Polygon(vec![[130.0, -260.0], [130.0, -190.0], [200.0, -190.0]]),
            4.0,
        ),
        text("pdf-name", page[0], 840.0, 76.0, "brand-guidelines.pdf"),
        small("pdf-note", page[0], 935.0, 56.0, "Not yet invented."),
    ]);
    let mut k = Shot::new(sc, elements)?;
    for (id, file) in [
        ("cave", "cave.webp"),
        ("wall", "wall.webp"),
        ("stencil-1", "stencil-1.webp"),
        ("stencil-2", "stencil-2.webp"),
        ("stencil-3", "stencil-3.webp"),
        ("hand-shadow", "hand-shadow.png"),
        ("hand", "hand.png"),
        ("hand-ochre", "hand-ochre.png"),
        ("mist", "mist.png"),
    ] {
        still(k.sc, id, file);
    }

    // The caves: a slow drift across the old panel.
    let dissolve = before(v.at("Someone"), 0.6);
    k.show(&["cave"], 0);
    k.init("cave.saturation", 0.85);
    k.init("cave.dim", 0.12);
    k.init("cave.focus-size", 0.9);
    k.init("cave.focus-x", 0.47);
    let drift = span(0, dissolve + s(1.2));
    k.glide("cave.focus-size", 0, 0.74, drift);
    k.glide("cave.focus-x", 0, 0.37, drift);
    k.init("scrim.blur", 48.0);
    k.appear("scrim", s(0.5), 0.6);
    k.appear("plate-note", s(0.6), 0.6);
    k.vanish("plate-note", dissolve, 0.4);

    // Closer: a bare wall, lit by the same lamp as the hand.
    let close = [
        "wall",
        "stencil-1",
        "stencil-2",
        "stencil-3",
        "hand-shadow",
        "hand",
        "hand-ochre",
        "mist",
    ];
    for id in close {
        k.init(&format!("{id}.dim"), 0.1);
        k.init(&format!("{id}.saturation"), 0.92);
    }
    k.appear("wall", dissolve, 1.0);
    k.hide(&["cave"], dissolve + s(1.05));
    k.glide("camera.zoom", dissolve, 1.06, 6.5);
    k.appear("reconstruction", dissolve + s(1.2), 0.6);

    // A hand comes in from the camera and presses flat against the rock.
    let hands = ["hand", "hand-ochre"];
    let contact = v.at("pressed") + s(0.08);
    let reach = before(contact, 0.62);
    for id in hands {
        k.arrive(id, reach, [60.0, 170.0], 0.62, Ease::CubicOut);
        k.tween(
            &format!("{id}.scale"),
            reach,
            [1.32, 1.0],
            0.62,
            Ease::CubicOut,
        );
        k.tween(
            &format!("{id}.rotation"),
            reach,
            [-0.06, 0.0],
            0.62,
            Ease::CubicOut,
        );
        k.tween(
            &format!("{id}.blur"),
            reach,
            [24.0, 0.0],
            0.5,
            Ease::Smootherstep,
        );
        // The palm flattens on contact.
        k.ease(
            &format!("{id}.scale"),
            contact,
            0.992,
            0.14,
            Ease::Smootherstep,
        );
        k.spring(&format!("{id}.scale"), contact + s(0.14), 1.0, 0.45);
    }
    k.appear("hand", reach, 0.28);
    k.tween("hand-shadow.x", reach, [70.0, 5.0], 0.62, Ease::CubicOut);
    k.tween("hand-shadow.y", reach, [-70.0, -5.0], 0.62, Ease::CubicOut);
    k.tween("hand-shadow.blur", reach, [30.0, 2.5], 0.62, Ease::CubicOut);
    k.fade("hand-shadow", reach, 0.85, 0.62);

    // Three breaths of ochre: each deposit spreads the cloud, coats the hand
    // a little more, and leaves a haze that settles.
    let breaths = [
        v.at("blew"),
        before(v.at("ochre"), 0.02),
        v.at("around") + s(0.04),
    ];
    for (i, &breath) in breaths.iter().enumerate() {
        k.ease(
            &format!("stencil-{}.opacity", i + 1),
            breath,
            1.0,
            0.34,
            Ease::CubicOut,
        );
        k.ease(
            "hand-ochre.opacity",
            breath,
            [0.45, 0.75, 1.0][i],
            0.34,
            Ease::CubicOut,
        );
        k.tween("mist.opacity", breath, [0.55, 0.0], 1.0, Ease::CubicOut);
        k.tween("mist.scale", breath, [0.97, 1.05], 1.0, Ease::CubicOut);
    }

    // The hand presses once more, then lifts away toward the camera.
    let away = v.at("taken");
    let lift = Ease::CubicBezier([0.4, 0.0, 0.75, 0.55]);
    for id in hands {
        let scale = format!("{id}.scale");
        k.ease(&scale, before(away, 0.28), 0.988, 0.2, Ease::Smootherstep);
        k.ease(&scale, before(away, 0.06), 1.42, 1.0, lift);
        k.shift(id, before(away, 0.06), [110.0, 260.0], 1.0, lift);
        k.ease(
            &format!("{id}.rotation"),
            before(away, 0.06),
            0.09,
            1.0,
            lift,
        );
        k.ease(&format!("{id}.blur"), before(away, 0.06), 30.0, 0.9, lift);
        k.vanish(id, away + s(0.32), 0.6);
    }
    let parting = Ease::CubicBezier([0.3, 0.0, 0.6, 1.0]);
    k.ease("hand-shadow.x", before(away, 0.06), 95.0, 0.75, parting);
    k.ease("hand-shadow.y", before(away, 0.06), -95.0, 0.75, parting);
    k.ease("hand-shadow.blur", before(away, 0.06), 42.0, 0.75, parting);
    k.vanish("hand-shadow", before(away, 0.06), 0.6);

    // A designer traces the absence, and the absence becomes a hole.
    let logo = v.at("logo");
    k.vanish("reconstruction", before(logo, 0.4), 0.4);
    k.vanish("scrim", before(logo, 0.4), 0.6);
    let traced = stencil.traced();
    k.draw_to("trace", logo, traced, 1.7);
    k.glide("camera.zoom", logo, 1.12, 2.8);
    let absence = v.at("what isn't there");
    k.appear("absence", absence, 0.8);
    k.vanish("trace", absence + s(0.5), 0.5);

    // The picture becomes a print on the page, beside the missing document.
    let all = v.at("All of this");
    k.glide("camera.zoom", all, 0.5, 1.9);
    k.glide("camera.x", all, 680.0, 1.9);
    let brand = v.at("brand");
    k.settle("page", brand);
    k.settle("fold", brand);
    k.type_in("pdf-name", v.at("guidelines"), 30.0);
    k.rise("pdf-note", before(v.at("which our"), 0.3));
    Ok(())
}

fn circle(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let c = [960.0, 500.0];
    let mut k = Shot::new(
        sc,
        vec![
            outline("circle", c, Figure::Circle(270.0), 2.5),
            solid("dot", c, Figure::Circle(5.0), Tone::Plain),
            compass_arm("arm", c, 270.0),
            note(
                "caption",
                960.0,
                965.0,
                "Euclid · Elements, Book I, Definition 15",
            ),
            aligned(
                small("curves", 958.0, 905.0, 32.0, "Curves:"),
                CaptionAlign::Right,
            ),
            aligned(text("one", 968.0, 905.0, 32.0, "1"), CaptionAlign::Left),
            aligned(text("zero", 968.0, 905.0, 32.0, "0"), CaptionAlign::Left),
            outline("o-outer", c, Figure::Rect([480.0, 600.0]), 2.0),
            outline("o-inner", c, Figure::Rect([240.0, 360.0]), 2.0),
            icon("ring", c, 600.0, RING, Tone::Plain),
        ],
    )?;
    k.rise("caption", s(0.25));
    k.vanish("caption", v.at("promise"), 0.6);
    k.pop("dot", v.at("circle"));
    let every = v.at("every");
    k.extend("arm", every);
    // The radius keeps its promise all the way round.
    let start = every + s(0.45);
    k.compass("circle", "arm", start, span(start, v.at("round") + s(0.3)));
    let release = v.at("no corners");
    k.retract("arm", release);
    k.vanish("dot", release + s(0.3), 0.4);
    k.rise("curves", release + s(0.35));
    k.rise("one", release + s(0.45));
    let admire = v.at("admired");
    k.ease("circle.emphasis", admire, 1.0, 1.2, Ease::Smootherstep);
    k.glide("camera.zoom", before(admire, 0.2), 1.08, 2.8);
    // With regret, the circle withdraws the way it came.
    let regret = v.at("regret");
    k.ease(
        "circle.draw",
        regret,
        0.0,
        1.5,
        Ease::CubicBezier([0.55, 0.0, 0.45, 1.0]),
    );
    k.ease("circle.emphasis", regret, 0.0, 1.0, Ease::Smootherstep);
    k.glide("camera.zoom", regret, 1.0, 2.0);
    let gone = regret + s(1.45);
    k.ease("one.y", gone, -26.0, 0.35, Ease::Smootherstep);
    k.vanish("one", gone, 0.3);
    k.set("zero.y", gone, 26.0);
    k.spring("zero.y", gone, 0.0, 0.55);
    k.appear("zero", gone, 0.3);
    let mark = before(v.at("The mark"), 0.1);
    k.draw("o-outer", mark, 1.0);
    k.draw("o-inner", mark + s(0.1), 0.95);
    k.appear("ring", mark + s(0.85), 0.45);
    k.vanish("o-outer", mark + s(1.3), 0.4);
    k.vanish("o-inner", mark + s(1.3), 0.4);
    Ok(())
}

/// The lens two side-by-side circles share, as a polygon about its center:
/// down circle B's arc from the top intersection, then up circle A's.
fn lens(a: [f32; 2], b: [f32; 2], radius: f32) -> Vec<[f32; 2]> {
    let center = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
    let half = (radius * radius - ((b[0] - a[0]) * 0.5).powi(2)).sqrt();
    let reach = (half / radius).asin();
    let arc = |origin: [f32; 2], from: f32| {
        (0..32).map(move |i| {
            let angle = from - 2.0 * reach * i as f32 / 31.0;
            [
                origin[0] + radius * angle.cos() - center[0],
                origin[1] + radius * angle.sin() - center[1],
            ]
        })
    };
    arc(b, std::f32::consts::PI + reach)
        .chain(arc(a, reach))
        .collect()
}
fn widened(mut element: StageElement, to: f32) -> StageElement {
    if let StageElement::Path { width, .. } = &mut element {
        *width = to;
    }
    element
}

fn vesica(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let r = 250.0;
    let a = [960.0, 500.0];
    let b = [1210.0, 500.0];
    let mid = [1085.0, 500.0];
    let below = [1085.0, 1722.0];
    let mut lens_shape = shape(
        "lens",
        mid,
        Figure::Polygon(lens(a, b, r)),
        Some(Fill::Tone(Tone::Muted)),
        None,
        0.25,
    );
    if let StageElement::Shape { fill_opacity, .. } = &mut lens_shape {
        *fill_opacity = 0.42;
    }
    let mut k = Shot::new(
        sc,
        vec![
            lens_shape,
            outline("circle-a", a, Figure::Circle(r), 2.2),
            outline("circle-b", b, Figure::Circle(r), 2.2),
            solid("dot-a", a, Figure::Circle(5.0), Tone::Plain),
            solid("dot-b", b, Figure::Circle(5.0), Tone::Plain),
            compass_arm("arm-b", b, r),
            text("name", mid[0], 845.0, 46.0, "Vesica piscis"),
            small(
                "translation",
                mid[0],
                900.0,
                28.0,
                "Latin: \u{201c}bladder of a fish\u{201d}",
            ),
            shape(
                "arch-left",
                b,
                Figure::Arc {
                    radius: r,
                    start: 0.75,
                    sweep: 1.0 / 6.0,
                },
                None,
                Some(Tone::Plain),
                3.0,
            ),
            shape(
                "arch-right",
                a,
                Figure::Arc {
                    radius: r,
                    start: 1.0 / 12.0,
                    sweep: 1.0 / 6.0,
                },
                None,
                Some(Tone::Plain),
                3.0,
            ),
            rule("jamb-left", a, [a[0], 700.0], 3.0, Tone::Plain),
            rule("jamb-right", b, [b[0], 700.0], 3.0, Tone::Plain),
            rule("sill", [a[0], 700.0], [b[0], 700.0], 3.0, Tone::Plain),
            note("arch-name", mid[0], 845.0, "Equilateral pointed arch"),
            outline(
                "frame-outer",
                [mid[0], 495.0],
                Figure::Rect([900.0, 660.0]),
                7.0,
            ),
            shape(
                "frame-inner",
                [mid[0], 495.0],
                Figure::Rect([840.0, 600.0]),
                None,
                Some(Tone::Muted),
                3.0,
            ),
            widened(
                guide(
                    "wire",
                    &[
                        [mid[0] - 300.0, 165.0],
                        [mid[0], 30.0],
                        [mid[0] + 300.0, 165.0],
                    ],
                    Arrow::None,
                ),
                3.0,
            ),
            solid("nail", [mid[0], 30.0], Figure::Circle(10.0), Tone::Plain),
            icon("o-plane", below, 1167.0, PLANE, Tone::Muted),
            icon("o-ring", below, 1167.0, RING, Tone::Plain),
        ],
    )?;
    k.show(&["circle-a", "dot-a"], 0);
    let second = v.at("second");
    k.pop("dot-b", second);
    k.glide("camera.x", second, 125.0, 2.3);
    k.extend("arm-b", v.at("circle"));
    let start = before(v.at("through"), 0.05);
    let sweep = span(start, v.at("first") + s(0.3));
    k.compass("circle-b", "arm-b", start, sweep);
    // B's circle passes through A's center three quarters of the way round.
    let crossing = start + s((sweep * reaches(COMPASS, 0.75)) as f64);
    k.set("dot-a.scale", crossing, 2.0);
    k.bounce("dot-a.scale", crossing, 1.0, 0.5, 0.3);
    k.retract("arm-b", start + s(sweep as f64 + 0.1));
    let overlap = v.at("overlap");
    k.appear("lens", overlap, 0.7);
    k.glide("camera.zoom", overlap, 1.05, 5.0);
    k.rise("name", v.at("vesica"));
    k.rise("translation", v.at("Latin"));
    // The translation is quietly withdrawn.
    let left = v.at("left in Latin") + s(0.1);
    k.vanish("translation", left, 0.7);
    k.ease("translation.y", left, 12.0, 0.7, Ease::Smootherstep);
    // The arch was inside the vesica all along.
    let gothic = v.at("Gothic");
    for id in ["circle-a", "circle-b"] {
        k.fade(id, gothic, 0.25, 0.7);
    }
    k.fade("lens", gothic, 0.14, 0.7);
    k.glide("camera.zoom", gothic, 1.0, 1.4);
    k.vanish("name", gothic, 0.4);
    let masons = v.at("masons");
    k.draw("arch-left", masons, 0.55);
    k.draw("arch-right", masons + s(0.55), 0.55);
    k.line("jamb-left", masons + s(1.1), 0.45);
    k.line("jamb-right", masons + s(1.1), 0.45);
    k.line("sill", masons + s(1.5), 0.4);
    k.rise("arch-name", v.at("arches"));
    // Framed, hung, and contemplated by the rectangle below.
    let framed = v.at("framed");
    k.vanish("arch-name", before(framed, 0.5), 0.4);
    // From the nail to the rectangle's foot, centered with room to spare.
    let pull = before(framed, 0.35);
    k.glide("camera.zoom", pull, 0.37, 1.7);
    k.glide("camera.y", pull, HANGING_Y, 1.7);
    k.draw("frame-outer", framed, 0.8);
    k.draw("frame-inner", framed + s(0.15), 0.8);
    let hung = v.at("hung");
    k.draw("wire", hung, 0.6);
    k.pop("nail", hung);
    let rectangle = before(v.at("rectangle"), 0.2);
    for id in ["o-plane", "o-ring"] {
        k.set(&format!("{id}.y"), rectangle, 90.0);
        k.spring(&format!("{id}.y"), rectangle, 0.0, 0.9);
        k.appear(id, rectangle, 0.5);
    }
    k.glide("camera.zoom", v.at("hope"), HANGING_ZOOM, 2.6);
    Ok(())
}
/// The vesica's closing camera, which the ratio's opening matches.
const HANGING_Y: f32 = 627.0;
const HANGING_ZOOM: f32 = 0.4;

/// One square cut from a golden rectangle, with the quarter arc through it.
struct Cut {
    center: [f32; 2],
    side: f32,
    pivot: [f32; 2],
    /// Where its arc starts, in turns clockwise from twelve o'clock.
    start: f32,
}

/// The squares of a portrait golden rectangle `width` wide, largest first,
/// cut bottom, right, top, left. Drawn clockwise, smallest first, their arcs
/// unfurl one continuous spiral outward.
fn golden_spiral(center: [f32; 2], width: f32, levels: usize) -> Vec<Cut> {
    let phi = (1.0 + 5f32.sqrt()) * 0.5;
    let [mut x, mut y] = [center[0] - width * 0.5, center[1] - width * phi * 0.5];
    let [mut w, mut h] = [width, width * phi];
    (0..levels)
        .map(|level| {
            let (corner, side, pivot, start) = match level % 4 {
                0 => {
                    let side = w;
                    let corner = [x, y + h - side];
                    h -= side;
                    (corner, side, corner, 0.25)
                }
                1 => {
                    let side = h;
                    let corner = [x + w - side, y];
                    w -= side;
                    (corner, side, [corner[0], corner[1] + side], 0.0)
                }
                2 => {
                    let side = w;
                    let corner = [x, y];
                    y += side;
                    h -= side;
                    (corner, side, [corner[0] + side, corner[1] + side], 0.75)
                }
                _ => {
                    let side = h;
                    let corner = [x, y];
                    x += side;
                    w -= side;
                    (corner, side, [corner[0] + side, corner[1]], 0.5)
                }
            };
            Cut {
                center: [corner[0] + side * 0.5, corner[1] + side * 0.5],
                side,
                pivot,
                start,
            }
        })
        .collect()
}

fn ratio(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let c = [960.0, 540.0];
    let width = 480.0;
    let phi = (1.0 + 5f32.sqrt()) * 0.5;
    let spiral = golden_spiral(c, width, 7);
    let mut elements = vec![
        icon("plane", c, 600.0, PLANE, Tone::Muted),
        icon("ring", c, 600.0, RING, Tone::Plain),
        // The golden construction lies over the mark.
        at_z(
            shape(
                "golden-frame",
                c,
                Figure::Rect([width, width * phi]),
                None,
                Some(Tone::Accent),
                2.0,
            ),
            -1.0,
        ),
    ];
    for (i, cut) in spiral.iter().enumerate() {
        elements.push(at_z(
            shape(
                &format!("square-{i}"),
                cut.center,
                Figure::Rect([cut.side, cut.side]),
                None,
                Some(Tone::Accent),
                1.2,
            ),
            -1.0,
        ));
        elements.push(at_z(
            shape(
                &format!("arc-{i}"),
                cut.pivot,
                Figure::Arc {
                    radius: cut.side.max(1.0),
                    start: cut.start,
                    sweep: 0.25,
                },
                None,
                Some(Tone::Accent),
                2.6,
            ),
            -1.0,
        ));
    }
    elements.extend([
        guide(
            "width-line",
            &[[720.0, 880.0], [1200.0, 880.0]],
            Arrow::Both,
        ),
        guide(
            "height-line",
            &[[1255.0, 240.0], [1255.0, 840.0]],
            Arrow::Both,
        ),
        small("width-label", 960.0, 918.0, 26.0, "240"),
        small("height-label", 1302.0, 540.0, 26.0, "300"),
        label(
            "actual",
            [420.0, 500.0, -1.0],
            150.0,
            "1.25",
            Tone::Plain,
            Face::SansBold,
        ),
        note("actual-note", 420.0, 610.0, "300 ÷ 240"),
        label(
            "phi",
            [1600.0, 500.0, -1.0],
            150.0,
            "1.618…",
            Tone::Accent,
            Face::Sans,
        ),
        note("phi-note", 1600.0, 610.0, "The golden ratio"),
        shape(
            "door",
            [1845.0, 600.0],
            Figure::Rect([110.0, 290.0]),
            None,
            Some(Tone::Muted),
            2.0,
        ),
    ]);
    let mut k = Shot::new(sc, elements)?;
    let shapes: Vec<String> = std::iter::once("golden-frame".to_string())
        .chain((0..spiral.len()).flat_map(|i| [format!("square-{i}"), format!("arc-{i}")]))
        .collect();
    let mut golden: Vec<String> = shapes.clone();
    golden.extend(["phi".into(), "phi-note".into()]);
    // Enter where the vesica left the rectangle, then settle to work.
    k.show(&["ring", "plane"], 0);
    let zoom = 1167.0 * HANGING_ZOOM / 600.0;
    let seen = 540.0 + (1722.0 - 540.0 - HANGING_Y) * HANGING_ZOOM;
    k.init("camera.zoom", zoom);
    k.init("camera.y", (540.0 - seen) / zoom);
    k.spring("camera.zoom", s(0.85), 1.0, 1.6);
    k.spring("camera.y", s(0.85), 0.0, 1.6);
    let first = v.at("golden");
    for id in ["ring", "plane"] {
        k.fade(id, first, 0.4, 0.6);
    }
    k.draw("golden-frame", first, 0.9);
    let measured = v.at("measured");
    for i in 0..spiral.len() {
        k.draw(&format!("square-{i}"), measured + s(0.2 * i as f64), 0.35);
    }
    // The spiral unfurls outward, with ceremony.
    let mut at = v.at("tremendous");
    for i in (0..spiral.len()).rev() {
        let seconds = 0.14 + 0.05 * (spiral.len() - 1 - i) as f32;
        k.draw(&format!("arc-{i}"), at, seconds);
        at += s(seconds as f64 * 0.92);
    }
    let mark = v.at("The mark");
    for id in &shapes {
        k.fade(id, mark, 0.25, 0.5);
    }
    for id in ["ring", "plane"] {
        k.fade(id, mark, 1.0, 0.5);
    }
    let exactly = v.at("exactly");
    k.draw("width-line", exactly, 0.6);
    k.rise("width-label", exactly + s(0.3));
    let quarter = v.at("quarter");
    k.draw("height-line", quarter, 0.6);
    k.rise("height-label", quarter + s(0.3));
    let taller = v.at("taller");
    k.rise("actual", taller);
    k.rise("actual-note", taller + s(0.25));
    let ideal = v.at_after("golden", "wide");
    for id in &shapes {
        k.fade(id, ideal, 1.0, 0.5);
    }
    for id in ["ring", "plane"] {
        k.fade(id, ideal, 0.5, 0.5);
        k.fade(id, v.at("shown"), 1.0, 0.8);
    }
    k.rise("phi", ideal + s(0.5));
    k.rise("phi-note", ideal + s(0.75));
    // It bows when thanked, then is shown out.
    let thanked = v.at("thanked");
    for id in &golden {
        let y = format!("{id}.y");
        k.ease(&y, thanked, 16.0, 0.28, Ease::Smootherstep);
        k.spring(&y, thanked + s(0.28), 0.0, 0.55);
    }
    let out = v.at("shown");
    k.draw("door", out, 0.5);
    let ids: Vec<&str> = golden.iter().map(String::as_str).collect();
    k.regroup(&ids, c, 0.2, [885.0, 60.0], out + s(0.25), 1.15);
    for id in &ids {
        k.vanish(id, out + s(1.0), 0.35);
    }
    k.vanish("door", out + s(1.6), 0.4);
    Ok(())
}

fn gesture(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let paper = [960.0, 520.0];
    let o = [940.0, 520.0];
    let size = 520.0;
    let u = size / 300.0;
    let [half_w, half_h, module] = [120.0 * u, 150.0 * u, 60.0 * u];
    // Each right angle is a quarter of the mark: two modules across and two
    // and a half down, so the four meet exactly.
    let corner = |id: &str, sx: f32, sy: f32| {
        let p = |x: f32, y: f32| [x * sx, y * sy];
        solid(
            id,
            [o[0] + sx * -half_w, o[1] + sy * -half_h],
            Figure::Polygon(vec![
                p(0.0, 0.0),
                p(2.0 * module, 0.0),
                p(2.0 * module, module),
                p(module, module),
                p(module, 2.5 * module),
                p(0.0, 2.5 * module),
            ]),
            Tone::Plain,
        )
    };
    let mut elements = vec![
        rounded(
            solid("paper", paper, Figure::Rect([560.0, 700.0]), Tone::Plain),
            3.0,
        ),
        at_z(
            StageElement::Footage {
                id: "giotto".into(),
                at: [paper[0], paper[1], 0.0],
                size: [600.0, 600.0],
                clip: Clip::new("giotto").decoded_at(60),
                fit: Fit::Fill,
                mask: Mask::default(),
                framed: false,
                tint: Tone::Plain,
            },
            0.0,
        ),
        note(
            "caption",
            960.0,
            965.0,
            "Vasari · Lives of the Artists, 1550",
        ),
        small("request", o[0], o[1], 30.0, "Request: proof of talent"),
        corner("corner-tl", 1.0, 1.0),
        corner("corner-tr", -1.0, 1.0),
        corner("corner-br", -1.0, -1.0),
        corner("corner-bl", 1.0, -1.0),
        icon("ring", o, size, RING, Tone::Plain),
    ];
    for i in 0..12 {
        let mut page = rounded(
            shape(
                &format!("page-{i}"),
                [1480.0 + i as f32 * 2.5, 540.0 - i as f32 * 3.5],
                Figure::Rect([300.0, 390.0]),
                Some(Fill::Material(Material::Surface)),
                Some(Tone::Muted),
                1.5,
            ),
            2.0,
        );
        if let StageElement::Shape { at, .. } = &mut page {
            at[2] = -(i as f32) * 0.01;
        }
        elements.push(page);
    }
    let top = [1480.0 + 11.0 * 2.5, 540.0 - 11.0 * 3.5];
    elements.extend([
        text("rationale", top[0], top[1] - 30.0, 34.0, "Rationale"),
        small("pages", top[0], top[1] + 22.0, 24.0, "112 pp."),
        shape(
            "spinner",
            [1392.0, 905.0],
            Figure::Arc {
                radius: 13.0,
                start: 0.0,
                sweep: 0.72,
            },
            None,
            Some(Tone::Plain),
            2.6,
        ),
        aligned(
            small("awaiting", 1420.0, 905.0, 26.0, "Awaiting reply"),
            CaptionAlign::Left,
        ),
    ]);
    let mut k = Shot::new(sc, elements)?;
    let drew = before(v.at("drew"), 0.05);
    let end = k.end();
    k.sc.media(footage::media(
        "giotto",
        "assets/giotto/%03d.png",
        drew,
        end,
    ));
    k.rise("caption", s(0.25));
    k.settle("paper", v.at("proof"));
    k.show(&["giotto"], drew);
    // Sent, and nothing else: it retires to the left as the exhibit to beat.
    let sent = v.at("sent nothing");
    for id in ["paper", "giotto"] {
        k.glide(&format!("{id}.x"), sent, -600.0, 1.2);
        k.glide(&format!("{id}.scale"), sent, 0.5, 1.2);
        k.glide(&format!("{id}.rotation"), sent, -0.035, 1.2);
    }
    k.vanish("caption", sent, 0.5);
    k.rise("request", v.at("asked"));
    // Four right angles arrive from the corners of the frame.
    let four = before(v.at("four right angles"), 0.15);
    k.vanish("request", before(four, 0.1), 0.35);
    let corners = [
        ("corner-tl", [-560.0, -380.0]),
        ("corner-tr", [560.0, -380.0]),
        ("corner-br", [560.0, 380.0]),
        ("corner-bl", [-560.0, 380.0]),
    ];
    for (i, (id, from)) in corners.iter().enumerate() {
        let at = four + s(0.09 * i as f64);
        k.set(&format!("{id}.x"), at, from[0]);
        k.set(&format!("{id}.y"), at, from[1]);
        k.bounce(&format!("{id}.x"), at, 0.0, 0.6, 0.16);
        k.bounce(&format!("{id}.y"), at, 0.0, 0.6, 0.16);
        k.appear(id, at, 0.15);
    }
    let whole = v.at("angles") + s(0.35);
    k.appear("ring", whole, 0.4);
    for (id, _) in corners {
        k.vanish(id, whole + s(0.4), 0.2);
    }
    // The rationale lands with some weight.
    let drop = before(v.at("rationale"), 0.45);
    for i in 0..12 {
        let at = drop + s(0.035 * i as f64);
        let id = format!("page-{i}");
        k.tween(&format!("{id}.y"), at, [-900.0, 0.0], 0.32, AWAY);
        k.show(&[&id], at);
    }
    let thud = drop + s(0.035 * 11.0 + 0.32);
    k.jolt(thud, [0.0, 1.0], 0.35);
    k.rise("rationale", thud + s(0.1));
    k.rise("pages", thud + s(0.25));
    let waiting = v.at("waiting");
    k.appear("spinner", waiting, 0.3);
    let turning = span(waiting, end);
    k.ease(
        "spinner.rotation",
        waiting,
        TAU * 1.1 * turning,
        turning,
        Ease::Linear,
    );
    k.rise("awaiting", waiting + s(0.15));
    Ok(())
}

fn inferno(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    const NUMERALS: [&str; 9] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"];
    let c = [960.0, 540.0];
    let deepest = 2700.0;
    let mut elements = vec![
        plate(
            "inferno",
            [960.0, 540.0, 10.0],
            [1920.0, 1080.0],
            Fit::Cover,
        ),
        scrim("scrim"),
        plate_note("plate-note", "Imagined plate · after Dante's Inferno"),
    ];
    for (i, numeral) in NUMERALS.iter().enumerate() {
        let z = 300.0 * i as f32;
        let radius = 470.0 - 24.0 * i as f32;
        elements.push(at_z(
            outline(&format!("ring-{i}"), c, Figure::Circle(radius), 2.2),
            z,
        ));
        elements.push(label(
            &format!("num-{i}"),
            [c[0], c[1] - radius - 26.0, z],
            30.0,
            numeral,
            Tone::Muted,
            Face::Sans,
        ));
    }
    elements.extend([
        at_z(art("fire", c, 380.0, DISC, [222, 108, 38]), deepest),
        at_z(art("ice", c, 380.0, DISC, [206, 228, 238]), deepest),
        label(
            "cocytus",
            [c[0], c[1] + 250.0, deepest],
            34.0,
            "Cocytus",
            Tone::Muted,
            Face::Sans,
        ),
        label(
            "onboarding",
            [960.0, 960.0, 299.0],
            25.0,
            "Onboarding: revised.",
            Tone::Muted,
            Face::Sans,
        ),
    ]);
    let mut k = Shot::new(sc, elements)?;
    still(k.sc, "inferno", "inferno.webp");
    k.show(&["inferno"], 0);
    k.init("inferno.saturation", 0.0);
    k.init("inferno.dim", 0.1);
    k.init("inferno.focus-size", 0.95);
    k.init("inferno.focus-y", 0.45);
    k.glide("inferno.focus-size", 0, 0.66, 3.2);
    k.glide("inferno.focus-y", 0, 0.6, 3.2);
    k.init("scrim.blur", 48.0);
    k.appear("scrim", s(0.2), 0.5);
    k.appear("plate-note", s(0.3), 0.5);
    let nine = v.at("nine circles");
    k.vanish("inferno", nine, 0.55);
    k.vanish("scrim", nine, 0.5);
    k.vanish("plate-note", nine, 0.4);
    for i in 0..9 {
        let at = nine + s(0.2 + 0.075 * i as f64);
        k.draw(&format!("ring-{i}"), at, 0.7);
        k.rise(&format!("num-{i}"), at + s(0.35));
    }
    // Light runs down the funnel, ring by ring.
    let narrower = v.at("narrower");
    for i in 0..9 {
        k.hit(
            &format!("ring-{i}.emphasis"),
            narrower + s(0.08 * i as f64),
            1.0,
        );
    }
    // Then the camera descends through them.
    let descend = v.at("descending");
    let frozen = v.at("frozen");
    k.glide("camera.z", descend, 2300.0, span(descend, frozen + s(0.2)));
    k.fade("fire", before(v.at("fire"), 0.1), 0.9, 0.45);
    k.vanish("fire", frozen, 0.7);
    k.appear("ice", frozen, 0.7);
    k.rise("cocytus", v.at("solid") + s(0.35));
    k.glide("camera.z", v.at("all the way down"), 2520.0, 2.0);
    // And comes briskly back up.
    let back = v.at("came back");
    k.vanish("cocytus", back, 0.3);
    k.ease("camera.z", back, 300.0, 1.5, Ease::Smootherstep);
    k.rise("onboarding", v.at("onboarding"));
    Ok(())
}

/// The mark's modules: 4 across, 5 down, 60 units each.
fn module_center(center: [f32; 2], module: f32, col: usize, row: usize) -> [f32; 2] {
    [
        center[0] + (col as f32 - 1.5) * module,
        center[1] + (row as f32 - 2.0) * module,
    ]
}
fn perimeter(col: usize, row: usize) -> bool {
    row == 0 || row == 4 || col == 0 || col == 3
}

fn grid(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let c = [960.0, 520.0];
    let size = 660.0;
    let m = size / 5.0;
    let [x0, y0] = [c[0] - 2.0 * m, c[1] - 2.5 * m];
    let mut elements = vec![];
    for k in 0..5 {
        let x = x0 + k as f32 * m;
        elements.push(guide(
            &format!("v-{k}"),
            &[[x, y0 - 30.0], [x, y0 + 5.0 * m + 30.0]],
            Arrow::None,
        ));
    }
    for k in 0..6 {
        let y = y0 + k as f32 * m;
        elements.push(guide(
            &format!("h-{k}"),
            &[[x0 - 30.0, y], [x0 + 4.0 * m + 30.0, y]],
            Arrow::None,
        ));
    }
    for row in 0..5 {
        for col in 0..4 {
            let at = module_center(c, m, col, row);
            let id = format!("cell-{col}-{row}");
            elements.push(if perimeter(col, row) {
                solid(&id, at, Figure::Rect([m, m]), Tone::Plain)
            } else {
                dashed(
                    shape(
                        &id,
                        at,
                        Figure::Rect([m - 14.0, m - 14.0]),
                        None,
                        Some(Tone::Muted),
                        1.5,
                    ),
                    [7.0, 6.0],
                )
            });
        }
    }
    let unit = module_center(c, m, 0, 0);
    let counter = [c[0] - m, c[0] + m, c[1] - 1.5 * m, c[1] + 1.5 * m];
    elements.extend([
        outline("unit", unit, Figure::Rect([m, m]), 2.5),
        text("unit-label", unit[0], unit[1], 40.0, "60"),
        icon("ring", c, size, RING, Tone::Plain),
        guide(
            "counter-width",
            &[
                [counter[0] + 16.0, counter[2] + 44.0],
                [counter[1] - 16.0, counter[2] + 44.0],
            ],
            Arrow::Both,
        ),
        guide(
            "counter-height",
            &[
                [counter[1] - 44.0, counter[2] + 70.0],
                [counter[1] - 44.0, counter[3] - 16.0],
            ],
            Arrow::Both,
        ),
        small("counter-w", c[0] - 20.0, counter[2] + 80.0, 26.0, "120"),
        small("counter-h", counter[1] - 90.0, c[1] + 30.0, 26.0, "180"),
    ]);
    let mut k = Shot::new(sc, elements)?;
    let lines: Vec<String> = (0..5)
        .map(|k| format!("v-{k}"))
        .chain((0..6).map(|k| format!("h-{k}")))
        .collect();
    let at = v.at("grid");
    for i in 0..5 {
        k.draw(&format!("v-{i}"), at + s(0.09 * i as f64), 0.7);
    }
    let five = before(v.at("five"), 0.2);
    for i in 0..6 {
        k.draw(&format!("h-{i}"), five + s(0.08 * i as f64), 0.7);
    }
    k.draw("unit", v.at("each module"), 0.6);
    k.rise("unit-label", v.at("sixty units"));
    // Fourteen modules become wall, clockwise from the top left.
    let fourteen = v.at("Fourteen");
    k.vanish("unit", before(fourteen, 0.15), 0.3);
    k.vanish("unit-label", before(fourteen, 0.15), 0.3);
    let walk = [
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
        (3, 4),
        (2, 4),
        (1, 4),
        (0, 4),
        (0, 3),
        (0, 2),
        (0, 1),
    ];
    for (i, (col, row)) in walk.iter().enumerate() {
        let id = format!("cell-{col}-{row}");
        let at = fourteen + s(0.055 * i as f64);
        k.appear(&id, at, 0.22);
        k.set(&format!("{id}.scale"), at, 0.84);
        k.bounce(&format!("{id}.scale"), at, 1.0, 0.5, 0.15);
    }
    let inner: Vec<String> = (1..4)
        .flat_map(|row| (1..3).map(move |col| format!("cell-{col}-{row}")))
        .collect();
    let remaining = v.at("remaining");
    for (i, id) in inner.iter().enumerate() {
        k.draw(id, remaining + s(0.07 * i as f64), 0.6);
    }
    let vital = v.at("vital");
    for (i, id) in inner.iter().enumerate() {
        k.hit(&format!("{id}.emphasis"), vital + s(0.05 * i as f64), 1.0);
    }
    // They do their work by not being there.
    let not = v.at("not being wall");
    for (i, id) in inner.iter().enumerate() {
        k.vanish(id, not + s(0.04 * i as f64), 0.7);
    }
    let careful = v.at("carefully");
    for (i, id) in lines.iter().enumerate() {
        k.vanish(id, careful + s(0.03 * i as f64), 0.6);
    }
    k.appear("ring", careful, 0.5);
    for (col, row) in walk {
        k.vanish(&format!("cell-{col}-{row}"), careful + s(0.55), 0.25);
    }
    let nothing = v.at("nothing");
    k.draw("counter-width", nothing, 0.6);
    k.draw("counter-height", nothing + s(0.15), 0.6);
    k.rise("counter-w", nothing + s(0.3));
    k.rise("counter-h", nothing + s(0.45));
    k.glide("camera.zoom", v.at("arranged"), 1.06, 2.8);
    Ok(())
}

fn counter(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let c = [960.0, 520.0];
    let size = 660.0;
    let m = size / 5.0;
    let counter = [c[0] - m, c[0] + m, c[1] - 1.5 * m, c[1] + 1.5 * m];
    let floor = c[1] - 0.5 * m;
    let slot = |i: usize| [960.0 + (i as f32 - 9.5) * 46.0, 820.0];
    let mut elements = vec![
        icon("plane", c, size, PLANE, Tone::Muted),
        icon("ring", c, size, RING, Tone::Plain),
        dashed(
            outline("counter-line", c, Figure::Rect([2.0 * m, 3.0 * m]), 1.6),
            [8.0, 6.0],
        ),
        guide(
            "leader",
            &[[counter[1] + 10.0, 360.0], [1250.0, 300.0], [1360.0, 300.0]],
            Arrow::None,
        ),
        aligned(
            text("counter-name", 1380.0, 300.0, 34.0, "Counter"),
            CaptionAlign::Left,
        ),
        guide(
            "bracket",
            &[[counter[0] - 22.0, floor], [counter[0] - 22.0, counter[3]]],
            Arrow::Both,
        ),
        small(
            "two-thirds",
            counter[0] - 58.0,
            (floor + counter[3]) * 0.5,
            26.0,
            "2/3",
        ),
        solid(
            "unit",
            [c[0], counter[2] + 14.0],
            Figure::Rect([22.0, 22.0]),
            Tone::Plain,
        ),
    ];
    for i in 0..20 {
        let at = slot(i);
        let id = format!("bar-{i}");
        elements.push(match i {
            0..14 => solid(&id, at, Figure::Rect([40.0, 40.0]), Tone::Plain),
            14..18 => solid(&id, at, Figure::Rect([40.0, 40.0]), Tone::Muted),
            _ => dashed(
                shape(
                    &id,
                    at,
                    Figure::Rect([38.0, 38.0]),
                    None,
                    Some(Tone::Muted),
                    1.5,
                ),
                [5.0, 4.0],
            ),
        });
    }
    let centre = |range: std::ops::Range<usize>| {
        let n = range.len() as f32;
        range.map(|i| slot(i)[0]).sum::<f32>() / n
    };
    let shares = [
        ("wall", centre(0..14), "70%", "wall"),
        ("floor", centre(14..18), "20%", "floor"),
        ("air", centre(18..20), "10%", "air"),
    ];
    for (id, x, share, name) in shares {
        elements.push(text(&format!("{id}-share"), x, 884.0, 30.0, share));
        elements.push(small(&format!("{id}-name"), x, 922.0, 22.0, name));
    }
    elements.push(small(
        "chapter",
        centre(18..20) - 40.0,
        980.0,
        22.0,
        "brand-guidelines.pdf · Chapter 7: Air",
    ));
    let mut k = Shot::new(sc, elements)?;
    k.show(&["ring"], 0);
    k.init("camera.zoom", 1.06);
    k.glide("camera.zoom", s(0.1), 1.0, 1.4);
    let named = v.at("counter");
    k.draw("counter-line", before(named, 0.25), 0.8);
    k.draw("leader", named, 0.5);
    k.rise("counter-name", named + s(0.35));
    // The floor rises into the lower two thirds.
    let floor_at = v.at("floor");
    for id in ["counter-line", "leader", "counter-name"] {
        k.vanish(id, floor_at, 0.4);
    }
    k.set("plane.y", before(floor_at, 0.1), 60.0);
    k.spring("plane.y", before(floor_at, 0.1), 0.0, 0.8);
    k.appear("plane", before(floor_at, 0.1), 0.5);
    let thirds = v.at("two thirds");
    k.draw("bracket", thirds, 0.5);
    k.rise("two-thirds", thirds + s(0.2));
    // Something is dropped in, and does not fall through.
    let nobody = v.at("nobody");
    k.appear("unit", nobody, 0.15);
    let rest = floor - 11.0 - (counter[2] + 14.0);
    let fall = nobody + s(0.25);
    k.ease("unit.y", fall, rest, 0.42, AWAY);
    let bounces = [(26.0, 0.18), (7.0, 0.09)];
    let mut at = fall + s(0.42);
    for (height, seconds) in bounces {
        k.ease("unit.y", at, rest - height, seconds, Ease::CubicOut);
        at += s(seconds as f64);
        k.ease("unit.y", at, rest, seconds, AWAY);
        at += s(seconds as f64);
    }
    // By area: the modules themselves are counted.
    let area = v.at("area");
    for id in ["unit", "bracket", "two-thirds"] {
        k.vanish(id, area, 0.3);
    }
    let lifted = [c[0], c[1] - 120.0];
    let scale = 0.72;
    k.regroup(&["ring", "plane"], c, scale, [0.0, -120.0], area, 1.0);
    let moved = m * scale;
    let mut wall = vec![];
    for row in 0..5 {
        for col in 0..4 {
            if perimeter(col, row) {
                wall.push(module_center(lifted, moved, col, row));
            }
        }
    }
    wall.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let floor_cells =
        [(1, 2), (1, 3), (2, 2), (2, 3)].map(|(col, row)| module_center(lifted, moved, col, row));
    let air_cells = [(1, 1), (2, 1)].map(|(col, row)| module_center(lifted, moved, col, row));
    let groups: [(Vec<[f32; 2]>, usize, u64); 3] = [
        (wall, 0, v.at("seventy")),
        (floor_cells.to_vec(), 14, v.at("twenty")),
        (air_cells.to_vec(), 18, v.at("ten")),
    ];
    let arc = Ease::CubicBezier([0.3, 0.0, 0.2, 1.0]);
    for (origins, first, at) in groups {
        for (j, origin) in origins.iter().enumerate() {
            let i = first + j;
            let id = format!("bar-{i}");
            let to = slot(i);
            let start = before(at, 0.1) + s(0.04 * j as f64);
            k.appear(&id, start, 0.12);
            k.tween(
                &format!("{id}.x"),
                start,
                [origin[0] - to[0], 0.0],
                0.8,
                Ease::Smootherstep,
            );
            k.tween(
                &format!("{id}.y"),
                start,
                [origin[1] - to[1], 0.0],
                0.8,
                arc,
            );
            k.tween(
                &format!("{id}.scale"),
                start,
                [moved / 40.0, 1.0],
                0.8,
                Ease::Smootherstep,
            );
        }
    }
    k.rise("wall-share", v.at("wall"));
    k.rise("wall-name", v.at("wall") + s(0.15));
    let floor_share = v.at_after("floor", "area");
    k.rise("floor-share", floor_share);
    k.rise("floor-name", floor_share + s(0.15));
    let air = v.at("clear air");
    k.rise("air-share", air);
    k.rise("air-name", air + s(0.15));
    k.type_in("chapter", v.at("chapter"), 34.0);
    Ok(())
}

fn word_letters() -> [&'static str; 8] {
    [
        "M3 2H1V5H3V2ZM4 6H0V1H4V6Z",
        "M1 5H3V2H1V5ZM4 6H1V7H0V1H4V6Z",
        "M4 4H1V5H4V6H0V1H4V4ZM1 3H3V2H1V3Z",
        "M3 2H1V6H0V1H3V2ZM4 6H3V2H4V6Z",
        "M4 2H1V5H4V6H0V1H4V2Z",
        "M3 2H1V5H3V2ZM4 6H0V1H4V6Z",
        "M3 2H1V5H3V2ZM4 6H0V1H3V0H4V6Z",
        "M1 2V3H3V2H1ZM4 4H1V5H4V6H0V1H4V4Z",
    ]
}
/// The OpenCode wordmark's letters, `size` square each, centered on `at`.
fn wordmark(prefix: &str, at: [f32; 2], size: f32) -> Vec<StageElement> {
    let unit = size / 8.0;
    word_letters()
        .into_iter()
        .enumerate()
        .map(|(i, path)| StageElement::Icon {
            id: format!("{prefix}-{i}"),
            at: [
                at[0] + (i as f32 - 3.5) * 5.0 * unit + 2.0 * unit,
                at[1],
                0.0,
            ],
            size,
            icon: String::new(),
            path: path.into(),
            view: 8.0,
            ink: None,
            tone: Tone::Plain,
        })
        .collect()
}

fn optics(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let c = [960.0, 520.0];
    let mut elements = vec![
        solid(
            "square",
            [760.0, 500.0],
            Figure::Rect([300.0, 300.0]),
            Tone::Plain,
        ),
        solid("disc", [1160.0, 500.0], Figure::Circle(150.0), Tone::Plain),
        guide("guide-top", &[[520.0, 350.0], [1400.0, 350.0]], Arrow::None),
        guide(
            "guide-bottom",
            &[[520.0, 650.0], [1400.0, 650.0]],
            Arrow::None,
        ),
        small("overshoot", 1160.0, 712.0, 24.0, "Optical overshoot · 3%"),
        solid("outer", c, Figure::Rect([480.0, 600.0]), Tone::Plain),
        shape(
            "inner",
            c,
            Figure::Rect([240.0, 360.0]),
            Some(Fill::Material(Material::Background)),
            None,
            0.25,
        ),
        icon("ring", c, 600.0, RING, Tone::Plain),
        note("legal", 960.0, 860.0, "cc: Legal"),
    ];
    elements.extend(wordmark("letter", c, 240.0));
    let mut k = Shot::new(sc, elements)?;
    k.settle("square", s(0.2));
    k.settle("disc", s(0.32));
    let measure = v.at("measure");
    k.draw("guide-top", measure, 0.7);
    k.draw("guide-bottom", measure + s(0.1), 0.7);
    // Equal heights; the circle looks smaller until it overshoots.
    let same = v.at_after("same", "look");
    k.bounce("disc.scale", same, 1.03, 0.7, 0.0);
    k.rise("overshoot", same + s(0.2));
    let walls = v.at("walls");
    for id in ["square", "disc", "guide-top", "guide-bottom", "overshoot"] {
        k.vanish(id, before(walls, 0.2), 0.4);
    }
    k.appear("outer", walls, 0.4);
    k.appear("inner", walls, 0.4);
    k.init("inner.scale", 1.0);
    k.glide("inner.scale", v.at("thickened"), 0.7, 0.9);
    k.glide("inner.scale", v.at("thinned"), 1.18, 0.8);
    let deliberate = v.at("deliberate");
    k.bounce("inner.scale", deliberate, 1.0, 0.8, 0.2);
    let exact = deliberate + s(0.85);
    k.appear("ring", exact, 0.25);
    k.hide(&["outer", "inner"], exact + s(0.3));
    let word = v.at("wordmark");
    k.vanish("ring", before(word, 0.35), 0.45);
    k.ease("ring.y", before(word, 0.35), -60.0, 0.5, Ease::Smootherstep);
    let letters: Vec<String> = (0..8).map(|i| format!("letter-{i}")).collect();
    for (i, id) in letters.iter().enumerate() {
        k.rise(id, word + s(0.05 * i as f64));
    }
    // Too generous: the letters drift apart as if leaving.
    let generous = v.at("generously");
    let leaving = v.at("leaving");
    let tight = v.at("tightly");
    for (i, id) in letters.iter().enumerate() {
        let x = format!("{id}.x");
        let spread = i as f32 - 3.5;
        k.glide(&x, generous, spread * 55.0, 1.1);
        k.glide(&x, leaving, spread * 100.0, 1.5);
        // Too tight: they slam together.
        k.ease(&x, tight, spread * -48.0, 0.42, AWAY);
    }
    k.jolt(tight + s(0.42), [0.0, 1.0], 0.3);
    k.rise("legal", v.at("legal"));
    let settled = v.at("merger") + s(0.75);
    for id in &letters {
        k.bounce(&format!("{id}.x"), settled, 0.0, 0.9, 0.15);
    }
    k.vanish("legal", settled, 0.5);
    Ok(())
}

fn pepsi(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let c = [680.0, 500.0];
    let mut elements = vec![
        shape(
            "globe",
            c,
            Figure::Circle(270.0),
            Some(Fill::Tone(Tone::Plain)),
            Some(Tone::Plain),
            2.0,
        ),
        art("red", c, 724.0, REFRESHMENT_RED, [214, 30, 46]),
        art("blue", c, 724.0, REFRESHMENT_BLUE, [0, 74, 152]),
        text("title-1", 1400.0, 330.0, 62.0, "The refreshment"),
        text("title-2", 1400.0, 408.0, 62.0, "hypothesis."),
        text("recognition", 1400.0, 630.0, 30.0, "Recognition: 100%"),
        note("as", 1400.0, 680.0, "Recognised as: someone else\u{2019}s"),
        label(
            "rejected",
            [960.0, 500.0, -2.0],
            160.0,
            "Rejected.",
            Tone::Plain,
            Face::SansBold,
        ),
        note("file", 960.0, 640.0, "Personnel file: closed."),
        small(
            "fiction",
            960.0,
            1032.0,
            22.0,
            "Speculative proposal · fictional personnel note",
        ),
    ];
    for i in 0..10 {
        elements.push(solid(
            &format!("dot-{i}"),
            [1238.0 + 36.0 * i as f32, 560.0],
            Figure::Circle(9.0),
            Tone::Plain,
        ));
    }
    let mut k = Shot::new(sc, elements)?;
    k.appear("fiction", s(0.3), 0.6);
    k.glide("camera.zoom", 0, 1.035, span(0, v.at("rejected")));
    let proposal = v.at("proposal");
    k.init("globe.fill", 0.0);
    k.draw("globe", proposal, 0.9);
    k.rise("title-1", proposal + s(0.2));
    k.rise("title-2", proposal + s(0.32));
    let red = v.at("red");
    k.arrive("red", red, [0.0, -40.0], 0.7, Ease::CubicOut);
    k.appear("red", red, 0.35);
    let blue = v.at("blue");
    k.arrive("blue", blue, [0.0, 40.0], 0.7, Ease::CubicOut);
    k.appear("blue", blue, 0.35);
    k.ease(
        "globe.fill",
        v.at("white smile"),
        1.0,
        0.6,
        Ease::Smootherstep,
    );
    let recognised = v.at("recognized");
    for i in 0..10 {
        k.pop(&format!("dot-{i}"), recognised + s(0.05 * i as f64));
    }
    k.rise("recognition", v.at("instantly"));
    k.rise("as", v.at_after("recognized", "asked") + s(0.2));
    // Rejected, with some force: only a ghost of the proposal remains.
    let rejected = v.at("rejected");
    // Opacity blends in linear light, so a ghost needs very little.
    for (id, ghost) in [
        ("globe", 0.035),
        ("red", 0.05),
        ("blue", 0.05),
        ("title-1", 0.025),
        ("title-2", 0.025),
        ("recognition", 0.0),
        ("as", 0.0),
    ] {
        k.fade(id, rejected, ghost, 0.25);
    }
    for i in 0..10 {
        k.vanish(&format!("dot-{i}"), rejected, 0.2);
    }
    k.show(&["rejected"], rejected);
    k.set("rejected.scale", rejected, 1.4);
    k.bounce("rejected.scale", rejected, 1.0, 0.35, 0.08);
    k.jolt(rejected + s(0.06), [0.0, 1.0], 0.55);
    k.rise("file", v.at("designer"));
    Ok(())
}

fn alternatives(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let slots = [330.0, 730.0, 1130.0, 1530.0];
    let y = 470.0;
    let mut elements = vec![];
    for (i, &x) in slots.iter().enumerate() {
        elements.push(shape(
            &format!("slot-{i}"),
            [x, y],
            Figure::Rect([300.0, 300.0]),
            None,
            Some(Tone::Muted),
            1.0,
        ));
    }
    elements.extend([
        art("card-red", [slots[0] - 52.0, y], 190.0, DISC, [235, 0, 27]),
        art(
            "card-yellow",
            [slots[0] + 52.0, y],
            190.0,
            DISC,
            [247, 158, 27],
        ),
        art("target-ring", [slots[1], y], 230.0, ANNULUS, [204, 0, 0]),
        art("target-dot", [slots[1], y], 76.0, DISC, [204, 0, 0]),
        art("opera", [slots[2], y], 280.0, OPERA, [255, 27, 45]),
        rounded(
            solid(
                "tile",
                [slots[3], y],
                Figure::Rect([210.0, 210.0]),
                Tone::Plain,
            ),
            48.0,
        ),
        art("tile-mark", [slots[3], y], 150.0, RING, BLACK),
        art("gold", [1830.0, y], 280.0, GOLDEN, [224, 179, 90]),
    ]);
    for (i, name) in [
        "Convergence",
        "Precision",
        "Operatic opening",
        "Softer boundary",
    ]
    .iter()
    .enumerate()
    {
        elements.push(small(&format!("name-{i}"), slots[i], 680.0, 24.0, name));
        elements.push(rule(
            &format!("strike-{i}"),
            [slots[i] - 125.0, y + 125.0],
            [slots[i] + 125.0, y - 125.0],
            3.0,
            Tone::Error,
        ));
    }
    let mut k = Shot::new(sc, elements)?;
    let end = k.end();
    k.glide("camera.zoom", 0, 1.03, span(0, end));
    let studies = v.at("studies");
    for i in 0..4 {
        k.draw(&format!("slot-{i}"), studies + s(0.1 * i as f64), 0.7);
    }
    let specimens: [(&str, &[&str]); 4] = [
        ("overlapping", &["card-red", "card-yellow"]),
        ("target", &["target-ring", "target-dot"]),
        ("oval", &["opera"]),
        ("rounded", &["tile", "tile-mark"]),
    ];
    for (i, (phrase, ids)) in specimens.iter().enumerate() {
        let at = v.at(phrase);
        for (j, id) in ids.iter().enumerate() {
            k.settle(id, at + s(0.08 * j as f64));
        }
        k.rise(&format!("name-{i}"), at + s(0.3));
    }
    // The golden rectangle tries the side door, and is shown out again.
    let golden = v.at("golden");
    k.appear("gold", golden, 0.2);
    k.set("gold.x", golden, 420.0);
    k.spring("gold.x", golden, 0.0, 0.9);
    let tried = v.at("tried");
    k.ease("gold.x", tried, 26.0, 0.25, Ease::Smootherstep);
    k.ease("gold.x", tried + s(0.25), -18.0, 0.35, Ease::Smootherstep);
    let door = v.at("side door");
    k.ease("gold.x", door, 460.0, 0.6, AWAY);
    k.vanish("gold", door + s(0.45), 0.15);
    let never = before(v.at("never"), 0.35);
    for i in 0..4 {
        k.line(&format!("strike-{i}"), never + s(0.12 * i as f64), 0.3);
    }
    Ok(())
}

fn finale(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let o = [960.0, 470.0];
    let size = 580.0;
    let u = size / 300.0;
    let mut elements = vec![
        rounded(
            plate("print", [960.0, 480.0, 0.0], [460.0, 259.0], Fit::Cover),
            4.0,
        ),
        outline(
            "compass-circle",
            [1480.0, 480.0],
            Figure::Circle(115.0),
            2.0,
        ),
        solid(
            "compass-dot",
            [1480.0, 480.0],
            Figure::Circle(4.0),
            Tone::Plain,
        ),
        compass_arm("compass-arm", [1480.0, 480.0], 115.0),
    ];
    for i in 0..9 {
        elements.push(outline(
            &format!("hell-{i}"),
            [2000.0, 480.0],
            Figure::Circle(125.0 - 13.0 * i as f32),
            1.3,
        ));
    }
    elements.extend([
        solid(
            "globe-base",
            [2520.0, 480.0],
            Figure::Circle(118.0),
            Tone::Plain,
        ),
        art(
            "globe-red",
            [2520.0, 480.0],
            316.0,
            REFRESHMENT_RED,
            [214, 30, 46],
        ),
        art(
            "globe-blue",
            [2520.0, 480.0],
            316.0,
            REFRESHMENT_BLUE,
            [0, 74, 152],
        ),
        outline("o-outer", o, Figure::Rect([240.0 * u, 300.0 * u]), 2.0),
        icon("plane", o, size, PLANE, Tone::Muted),
        icon("ring", o, size, RING, Tone::Plain),
        dashed(
            outline(
                "hairline",
                o,
                Figure::Rect([120.0 * u - 28.0, 180.0 * u - 28.0]),
                1.5,
            ),
            [6.0, 5.0],
        ),
    ]);
    elements.extend(wordmark("letter", [960.0, 880.0], 104.0));
    let credits = [
        (
            "credit-1",
            430.0,
            "Contemporary formal studies of the OpenCode mark.",
        ),
        (
            "credit-2",
            485.0,
            "Imagined plates and reconstructions. Fictional proposals and personnel notes.",
        ),
        (
            "credit-3",
            590.0,
            "Sources: Euclid · Vasari · Dante · Chauvet · Lascaux",
        ),
        ("credit-4", 645.0, "Exact artwork: opencode.ai/brand"),
    ];
    for (id, y, line) in credits {
        elements.push(note(id, 960.0, y, line));
    }
    let mut k = Shot::new(sc, elements)?;
    still(k.sc, "print", "stencil-3.webp");
    let strip = [
        "print",
        "compass-circle",
        "compass-dot",
        "globe-base",
        "globe-red",
        "globe-blue",
    ];
    // A filmstrip of everything so far.
    k.settle("print", before(v.at("caves"), 0.1));
    let compass = v.at("compass");
    k.glide("camera.x", before(compass, 0.1), 520.0, 0.7);
    k.pop("compass-dot", compass);
    k.extend("compass-arm", compass);
    k.compass("compass-circle", "compass-arm", compass + s(0.15), 0.8);
    k.retract("compass-arm", compass + s(1.0));
    let nine = v.at("nine circles");
    k.glide("camera.x", before(nine, 0.1), 1040.0, 0.7);
    for i in 0..9 {
        k.draw(&format!("hell-{i}"), nine + s(0.05 * i as f64), 0.5);
    }
    let drinks = v.at("soft drinks");
    k.glide("camera.x", before(drinks, 0.15), 1560.0, 0.7);
    for (i, id) in ["globe-base", "globe-red", "globe-blue"].iter().enumerate() {
        k.settle(id, before(drinks, 0.1) + s(0.06 * i as f64));
    }
    // Then all the way back to where it began.
    let arrived = v.at("arrived");
    k.glide("camera.x", arrived, 0.0, 1.0);
    for id in strip
        .iter()
        .map(|id| id.to_string())
        .chain((0..9).map(|i| format!("hell-{i}")))
    {
        k.vanish(&id, arrived + s(0.1), 0.45);
    }
    let rectangle = before(v.at("rectangle"), 0.15);
    k.draw("o-outer", rectangle, 0.8);
    k.appear("ring", rectangle + s(0.65), 0.5);
    k.vanish("o-outer", rectangle + s(1.15), 0.4);
    let hole = v.at("hole");
    k.draw("hairline", hole, 0.8);
    let important = v.at("most important");
    k.glide("camera.zoom", important, 1.1, 2.4);
    k.glide("camera.y", important, -40.0, 2.4);
    let name = v.at_any(&["Open code", "OpenCode"]);
    k.vanish("hairline", before(name, 0.2), 0.4);
    k.set("plane.y", name, 50.0);
    k.spring("plane.y", name, 0.0, 0.8);
    k.appear("plane", name, 0.5);
    for i in 0..8 {
        k.rise(&format!("letter-{i}"), name + s(0.1 + 0.04 * i as f64));
    }
    // "Do come in": the camera aligns with the opening and goes through it.
    let counter_top = o[1] - 90.0 * u;
    let floor_top = o[1] - 150.0 * u + 120.0 * u;
    let doorway = (counter_top + floor_top) * 0.5;
    k.glide("camera.y", v.at("Do"), doorway - 540.0, 0.55);
    let come = v.at("come in");
    k.ease("camera.zoom", come, 16.0, 1.1, THROUGH);
    let through = come + s(1.1);
    k.set("camera.zoom", through, 1.0);
    k.set("camera.y", through, 0.0);
    let shown: Vec<String> = ["ring", "plane"]
        .map(String::from)
        .into_iter()
        .chain((0..8).map(|i| format!("letter-{i}")))
        .collect();
    for id in &shown {
        k.set(&format!("{id}.opacity"), through, 0.0);
    }
    for (i, (id, _, _)) in credits.iter().enumerate() {
        k.rise(id, through + s(0.6 + 0.18 * i as f64));
    }
    Ok(())
}

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let media_root = root.join("../../output/shape-of-openness");
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "target/shape-of-openness/reel.json".into());
    let narration = Narration::load(&root.join("narration"))?;
    let stencil = Stencil::load()?;
    let mut segments = vec![];
    for (id, lead, tail, enter) in PARTS {
        let clip = narration.clip(id)?;
        let duration = s(lead) + clip.duration() + s(tail);
        let mut sc = PlanBuilder::new(id, duration);
        let spoken = clip.place(&mut sc, s(lead));
        match id {
            "opening" => opening(&mut sc, &spoken)?,
            "cave" => cave(&mut sc, &spoken, &stencil)?,
            "circle" => circle(&mut sc, &spoken)?,
            "vesica" => vesica(&mut sc, &spoken)?,
            "ratio" => ratio(&mut sc, &spoken)?,
            "gesture" => gesture(&mut sc, &spoken)?,
            "inferno" => inferno(&mut sc, &spoken)?,
            "grid" => grid(&mut sc, &spoken)?,
            "counter" => counter(&mut sc, &spoken)?,
            "optics" => optics(&mut sc, &spoken)?,
            "pepsi" => pepsi(&mut sc, &spoken)?,
            "alternatives" => alternatives(&mut sc, &spoken)?,
            "finale" => finale(&mut sc, &spoken)?,
            _ => unreachable!(),
        }
        sc.cue(id, 0, duration);
        sc.sort_events();
        let mut plan = sc.finish()?;
        for media in &mut plan.media {
            if media.path.is_relative() {
                media.path = media_root.join(&media.path);
            }
        }
        segments.push(match enter {
            Enter::Cut => ReelSegmentPlan::cut(plan),
            Enter::Dip(seconds) => ReelSegmentPlan::dipped(plan, s(seconds)),
            Enter::Dissolve(seconds) => ReelSegmentPlan::crossfaded(plan, s(seconds)),
        });
    }
    let reel = ReelPlan::new("the-shape-of-openness", segments)?;
    let parent = output.parent().unwrap();
    fs::create_dir_all(parent)?;
    for segment in &reel.segments {
        segment
            .plan
            .write_or_print(Some(parent.join(format!("{}.json", segment.plan.id))))?;
    }
    fs::write(&output, serde_json::to_string_pretty(&reel)? + "\n")?;
    println!("{}", output.display());
    Ok(())
}
