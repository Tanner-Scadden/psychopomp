//! A flat editorial film. The narration supplies the clock; exact artwork,
//! geometric studies, and image plates supply the pictures. No particle forms.
use anyhow::Result;
use psychopomp::{
    author::{PlanBuilder, seconds},
    face::Face,
    footage::{self, Clip, Fit, Mask},
    math::easing::Ease,
    narration::{Narration, Spoken},
    plan::{ReelPlan, ReelSegmentPlan, ReelTransitionStyle, ScenePlan},
    stage::{Arrow, Figure, Fill, StageActor, StageElement, StagePlan, StagePost},
    tone::Tone,
};
use std::{fs, path::PathBuf};

// Official paths translated +30 on x to center their 240×300 artwork in a 300-square view.
const RING: &str = "M210 60H90V240H210V60ZM270 300H30V0H270V300Z";
const PLANE: &str = "M210 240H90V120H210V240Z";
const IDS: [&str; 13] = [
    "opening",
    "cave",
    "circle",
    "vesica",
    "ratio",
    "gesture",
    "inferno",
    "grid",
    "counter",
    "optics",
    "pepsi",
    "alternatives",
    "finale",
];

fn text(id: &str, x: f32, y: f32, size: f32, value: &str) -> StageElement {
    StageElement::label(id, [x, y, -1.0], size, &[(value, Tone::Plain)]).face(Face::Sans)
}
fn note(id: &str, x: f32, y: f32, value: &str) -> StageElement {
    StageElement::label(id, [x, y, -1.0], 25.0, &[(value, Tone::Muted)]).face(Face::Sans)
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
fn pigment(mut element: StageElement, color: [u8; 3]) -> StageElement {
    if let StageElement::Icon { ink, .. } = &mut element {
        *ink = Some(color);
    }
    element
}
fn mark(prefix: &str, x: f32, y: f32, size: f32) -> Vec<StageElement> {
    vec![
        icon(&format!("{prefix}-plane"), [x, y], size, PLANE, Tone::Muted),
        icon(&format!("{prefix}-ring"), [x, y], size, RING, Tone::Plain),
    ]
}
fn shape(
    id: &str,
    at: [f32; 2],
    figure: Figure,
    fill: Option<Tone>,
    stroke: Option<Tone>,
    width: f32,
) -> StageElement {
    StageElement::Shape {
        id: id.into(),
        at: [at[0], at[1], 0.0],
        shape: figure,
        corner: 0.0,
        fill: fill.map(Fill::Tone),
        fill_opacity: 1.0,
        stroke,
        width: width.max(0.25),
        dash: None,
        arrow: Arrow::None,
    }
}
fn circle(id: &str, x: f32, y: f32, r: f32) -> StageElement {
    shape(id, [x, y], Figure::Circle(r), None, Some(Tone::Plain), 2.0)
}
fn rect(id: &str, x: f32, y: f32, w: f32, h: f32, tone: Tone) -> StageElement {
    shape(id, [x, y], Figure::Rect([w, h]), Some(tone), None, 0.0)
}
fn photo(id: &str) -> StageElement {
    StageElement::Footage {
        id: id.into(),
        at: [960.0, 540.0, 10.0],
        size: [1920.0, 1080.0],
        clip: Clip::new(id),
        fit: Fit::Cover,
        mask: Mask::default(),
        framed: false,
        tint: Tone::Plain,
    }
}
fn show(s: &mut StageActor, sc: &mut PlanBuilder, at: u64, ids: &[&str]) {
    for id in ids {
        s.set(sc, &format!("{id}.opacity"), at, 1.0);
    }
}
fn hide(s: &mut StageActor, sc: &mut PlanBuilder, at: u64, ids: &[&str]) {
    for id in ids {
        s.set(sc, &format!("{id}.opacity"), at, 0.0);
    }
}
fn draw(s: &mut StageActor, sc: &mut PlanBuilder, at: u64, id: &str, duration: f32) {
    show(s, sc, at, &[id]);
    s.set(sc, &format!("{id}.draw"), at, 0.0);
    s.ease(
        sc,
        &format!("{id}.draw"),
        at,
        1.0,
        duration,
        Ease::CubicBezier([0.45, 0.0, 0.2, 1.0]),
    );
}
fn stage(sc: &mut PlanBuilder, elements: Vec<StageElement>) -> Result<StageActor> {
    let plan = StagePlan {
        post: StagePost::FLAT,
        elements,
    };
    let mut s = StageActor::declare(sc, "stage", &plan)?;
    for element in &plan.elements {
        s.channel(sc, &format!("{}.opacity", element.id()), 0.0);
    }
    Ok(s)
}
fn cut_all(s: &mut StageActor, sc: &mut PlanBuilder, at: u64, ids: &[&str]) {
    for id in ids {
        s.set(sc, &format!("{id}.opacity"), at, 0.0);
    }
}

fn opening(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut elements = mark("o", 960.0, 510.0, 620.0);
    elements.extend([
        note("boundary", 960.0, 950.0, "An opening requires a boundary."),
        text("presence", 410.0, 520.0, 62.0, "Presence."),
        text("absence", 1510.0, 520.0, 62.0, "Absence."),
        text("title-1", 960.0, 422.0, 126.0, "The shape"),
        text("title-2", 960.0, 565.0, 126.0, "of openness."),
        note("byline", 960.0, 790.0, "An inquiry into the OpenCode O"),
    ]);
    let mut s = stage(sc, elements)?;
    show(&mut s, sc, 0, &["o-ring", "boundary"]);
    s.channel(sc, "camera.zoom", 1.18);
    s.glide(sc, "camera.zoom", 0, 1.0, 1.8);
    s.channel(sc, "o-ring.scale", 0.9);
    s.to(sc, "o-ring.scale", 0, 1.0, 1.4);
    show(&mut s, sc, v.at("presence"), &["presence"]);
    show(&mut s, sc, v.at("absence"), &["absence"]);
    let at = v.at("This is");
    hide(
        &mut s,
        sc,
        at,
        &["o-ring", "boundary", "presence", "absence"],
    );
    show(&mut s, sc, at, &["title-1", "title-2", "byline"]);
    Ok(())
}

fn cave(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let hand = "M112 278L105 208L72 173Q60 158 72 148Q83 141 96 156L120 178L108 82Q106 66 119 63Q132 63 135 80L148 154L145 51Q145 33 158 33Q173 33 173 51L176 150L187 61Q190 44 203 48Q216 51 213 68L204 163L220 98Q225 84 238 90Q250 96 244 113L222 217L209 278Z";
    let mut s = stage(
        sc,
        vec![
            photo("cave"),
            note(
                "imagined",
                960.0,
                1010.0,
                "Contemporary imagined plate · Chauvet / Lascaux studies",
            ),
            icon(
                "stencil",
                [960.0, 530.0],
                690.0,
                &format!("M0 0V300H300V0Z{hand}"),
                Tone::Muted,
            ),
            icon("hand", [960.0, 530.0], 690.0, hand, Tone::Plain),
            text(
                "negative",
                960.0,
                910.0,
                48.0,
                "An absence becomes an image.",
            ),
            text("pdf", 960.0, 535.0, 104.0, "brand-guidelines.pdf"),
            note("unavailable", 960.0, 680.0, "Not yet available."),
        ],
    )?;
    show(&mut s, sc, 0, &["cave", "imagined"]);
    s.channel(sc, "cave.saturation", 0.22);
    s.glide(sc, "cave.focus-size", 0, 0.76, 7.0);
    s.glide(sc, "cave.focus-x", 0, 0.38, 7.0);
    let at = v.at("A hand");
    hide(&mut s, sc, at, &["cave", "imagined"]);
    show(&mut s, sc, at, &["hand"]);
    let withdraw = v.at("withdraws");
    show(&mut s, sc, withdraw, &["stencil"]);
    s.glide(sc, "hand.x", withdraw, 130.0, 0.65);
    s.glide(sc, "hand.opacity", withdraw, 0.0, 0.65);
    show(&mut s, sc, v.at("An absence"), &["negative"]);
    let pdf = v.at("brand guidelines");
    hide(&mut s, sc, pdf, &["negative", "stencil"]);
    show(&mut s, sc, pdf, &["pdf", "unavailable"]);
    Ok(())
}

fn circles(sc: &mut PlanBuilder, v: &Spoken<'_>, vesica: bool) -> Result<()> {
    let mut elements = vec![
        circle("a", if vesica { 810.0 } else { 960.0 }, 510.0, 290.0),
        circle("b", 1100.0, 510.0, 290.0),
        rect("centre", 960.0, 510.0, 6.0, 6.0, Tone::Plain),
        shape(
            "radius",
            [960.0, 510.0],
            Figure::Polygon(vec![
                [0.0, -0.75],
                [290.0, -0.75],
                [290.0, 0.75],
                [0.0, 0.75],
            ]),
            Some(Tone::Muted),
            None,
            0.25,
        ),
        text(
            "caption",
            960.0,
            938.0,
            40.0,
            if vesica {
                "Vesica piscis"
            } else {
                "One centre. One distance."
            },
        ),
        text("zero", 960.0, 950.0, 45.0, "Curves: 0"),
        note(
            "relation",
            960.0,
            165.0,
            "Two equal radii · centre separation = radius",
        ),
    ];
    elements.extend(mark("o", 960.0, 500.0, 600.0));
    let mut s = stage(sc, elements)?;
    draw(&mut s, sc, 0, "a", 1.2);
    show(&mut s, sc, 0, &["caption"]);
    if vesica {
        s.channel(sc, "b.x", 280.0);
        s.glide(
            sc,
            "b.x",
            v.at_any(&["Each centre", "Each center"]),
            0.0,
            1.0,
        );
        draw(
            &mut s,
            sc,
            v.at_any(&["Each centre", "Each center"]),
            "b",
            1.0,
        );
        show(&mut s, sc, v.at("third space"), &["relation"]);
        s.glide(sc, "camera.zoom", v.at("contemplation"), 1.25, 1.5);
        let at = v.at("rectangle");
        hide(&mut s, sc, at, &["a", "b", "caption", "relation"]);
        s.set(sc, "camera.zoom", at, 1.0);
        show(&mut s, sc, at, &["o-ring", "o-plane"]);
    } else {
        show(
            &mut s,
            sc,
            v.at_any(&["One centre", "One center"]),
            &["centre"],
        );
        show(&mut s, sc, v.at("One distance"), &["radius"]);
        s.ease(
            sc,
            "radius.rotation",
            v.at("One distance"),
            std::f32::consts::TAU,
            1.8,
            Ease::Smootherstep,
        );
        let at = v.at("Our final mark");
        hide(&mut s, sc, at, &["a", "centre", "radius", "caption"]);
        show(&mut s, sc, at, &["o-ring", "o-plane", "zero"]);
    }
    Ok(())
}

fn ratio(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut s = stage(
        sc,
        vec![
            text("phi", 960.0, 495.0, 228.0, "1.618…"),
            note("golden", 960.0, 770.0, "The golden ratio"),
            text("actual", 960.0, 495.0, 228.0, "1.25").face(Face::SansBold),
            note("five", 960.0, 770.0, "5 ÷ 4"),
            text("thanks", 960.0, 510.0, 108.0, "Thank you for your time."),
        ],
    )?;
    show(&mut s, sc, 0, &["phi", "golden"]);
    s.glide(sc, "phi.scale", 0, 1.08, 3.0);
    let actual = v.at("actual mark");
    hide(&mut s, sc, actual, &["phi", "golden"]);
    show(&mut s, sc, actual, &["actual", "five"]);
    let end = v.at("thanked");
    hide(&mut s, sc, end, &["actual", "five"]);
    show(&mut s, sc, end, &["thanks"]);
    Ok(())
}

fn gesture(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut elements = vec![
        text("giotto", 960.0, 510.0, 140.0, "Giotto."),
        circle("circle", 960.0, 510.0, 280.0),
        note(
            "vasari",
            960.0,
            940.0,
            "Giotto's circle · as recounted by Vasari",
        ),
        note("corners", 960.0, 940.0, "Four right angles."),
    ];
    for (i, (x, y, w, h)) in [
        (750.0, 500.0, 70.0, 520.0),
        (1170.0, 500.0, 70.0, 520.0),
        (960.0, 275.0, 350.0, 70.0),
        (960.0, 725.0, 350.0, 70.0),
    ]
    .into_iter()
    .enumerate()
    {
        elements.push(rect(&format!("edge-{i}"), x, y, w, h, Tone::Plain));
    }
    let mut s = stage(sc, elements)?;
    show(&mut s, sc, 0, &["vasari", "giotto"]);
    hide(&mut s, sc, v.at("circle"), &["giotto"]);
    draw(&mut s, sc, v.at("circle"), "circle", 1.6);
    let at = v.at("four right angles");
    hide(&mut s, sc, at, &["circle", "vasari"]);
    show(&mut s, sc, at, &["corners"]);
    for i in 0..4 {
        show(
            &mut s,
            sc,
            at + seconds(i as f64 * 0.12),
            &[&format!("edge-{i}")],
        );
    }
    Ok(())
}

fn inferno(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut elements = vec![
        photo("inferno"),
        note(
            "plate",
            960.0,
            1010.0,
            "Contemporary imagined plate · Dante's Inferno",
        ),
        text("nine", 1510.0, 480.0, 182.0, "IX"),
        note("architecture", 1510.0, 700.0, "An architecture of descent"),
        text("ice", 960.0, 470.0, 216.0, "Ice."),
        note("onboarding", 960.0, 750.0, "Onboarding review pending."),
    ];
    for i in 0..9 {
        elements.push(circle(
            &format!("ring-{i}"),
            670.0,
            530.0,
            380.0 - i as f32 * 40.0,
        ));
    }
    let mut s = stage(sc, elements)?;
    show(&mut s, sc, 0, &["inferno", "plate"]);
    s.channel(sc, "inferno.saturation", 0.0);
    s.glide(sc, "inferno.focus-size", 0, 0.72, 5.0);
    s.glide(sc, "inferno.focus-y", 0, 0.36, 5.0);
    let at = v.at("nine circles");
    hide(&mut s, sc, at, &["inferno", "plate"]);
    show(&mut s, sc, at, &["nine", "architecture"]);
    for i in 0..9 {
        draw(
            &mut s,
            sc,
            at + seconds(i as f64 * 0.09),
            &format!("ring-{i}"),
            0.65,
        );
    }
    let centre = v.at_any(&["centre", "center"]);
    for i in 0..9 {
        hide(&mut s, sc, centre, &[&format!("ring-{i}")]);
    }
    hide(&mut s, sc, centre, &["nine", "architecture"]);
    show(&mut s, sc, v.at("ice"), &["ice"]);
    show(&mut s, sc, v.at("onboarding"), &["onboarding"]);
    Ok(())
}

fn grid(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut elements = vec![
        text("grid-title", 960.0, 510.0, 140.0, "The grid."),
        text("four", 1440.0, 420.0, 128.0, "4 × 5"),
        note("sixty", 1440.0, 650.0, "1u = 60"),
        note("inside", 1440.0, 730.0, "14 perimeter / 6 interior"),
    ];
    for row in 0..5 {
        for col in 0..4 {
            let border = row == 0 || row == 4 || col == 0 || col == 3;
            elements.push(shape(
                &format!("cell-{col}-{row}"),
                [410.0 + col as f32 * 122.0, 260.0 + row as f32 * 122.0],
                Figure::Rect([120.0, 120.0]),
                border.then_some(Tone::Plain),
                Some(Tone::Muted),
                1.0,
            ));
        }
    }
    let mut s = stage(sc, elements)?;
    let begin = v.at("Four modules");
    show(&mut s, sc, 0, &["grid-title"]);
    hide(&mut s, sc, begin, &["grid-title"]);
    for row in 0..5 {
        for col in 0..4 {
            show(
                &mut s,
                sc,
                begin + seconds((row * 4 + col) as f64 * 0.045),
                &[&format!("cell-{col}-{row}")],
            );
        }
    }
    show(&mut s, sc, v.at("Five modules"), &["four"]);
    show(&mut s, sc, v.at("Sixty units"), &["sixty"]);
    show(&mut s, sc, v.at("Fourteen modules"), &["inside"]);
    let within = v.at("Six within");
    for row in 1..4 {
        for col in 1..3 {
            s.glide(sc, &format!("cell-{col}-{row}.scale"), within, 0.88, 0.5);
        }
    }
    Ok(())
}

fn counter(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut elements = mark("o", 610.0, 500.0, 650.0);
    elements.extend([
        text("seventy", 1360.0, 320.0, 100.0, "70%"),
        note("perimeter", 1360.0, 445.0, "Perimeter"),
        text("twenty", 1360.0, 530.0, 100.0, "20%"),
        note("secondary", 1360.0, 655.0, "Secondary ink"),
        text("ten", 1360.0, 750.0, 100.0, "10%"),
        note("transparency", 1360.0, 875.0, "Transparency"),
        note(
            "spec",
            610.0,
            955.0,
            "Even our nothing has a specification.",
        ),
    ]);
    let mut s = stage(sc, elements)?;
    show(&mut s, sc, 0, &["o-ring"]);
    s.channel(sc, "camera.x", -350.0);
    s.channel(sc, "camera.y", -40.0);
    s.channel(sc, "camera.zoom", 1.35);
    let pullback = v.at("Seventy").saturating_sub(seconds(0.85));
    s.glide(sc, "camera.x", pullback, 0.0, 0.85);
    s.glide(sc, "camera.y", pullback, 0.0, 0.85);
    s.glide(sc, "camera.zoom", pullback, 1.0, 0.85);
    show(&mut s, sc, v.at("secondary plane"), &["o-plane"]);
    s.channel(sc, "o-plane.y", 45.0);
    s.to(sc, "o-plane.y", v.at("secondary plane"), 0.0, 0.7);
    show(&mut s, sc, v.at("Seventy"), &["seventy", "perimeter"]);
    show(&mut s, sc, v.at("Twenty"), &["twenty", "secondary"]);
    show(&mut s, sc, v.at("Ten"), &["ten", "transparency"]);
    show(&mut s, sc, v.at("nothing"), &["spec"]);
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
fn optics(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut elements = vec![
        text("equal", 960.0, 470.0, 100.0, "Equality ≠ equilibrium"),
        note("interval", 960.0, 940.0, "The interval enters language."),
    ];
    for (i, weight) in [25, 60, 95].into_iter().enumerate() {
        let d = format!(
            "M30 0H270V300H30ZM{} {}V{}H{}V{}Z",
            30 + weight,
            weight,
            300 - weight,
            270 - weight,
            weight
        );
        elements.push(icon(
            &format!("weight-{i}"),
            [960.0, 490.0],
            650.0,
            &d,
            Tone::Plain,
        ));
    }
    for (i, path) in word_letters().into_iter().enumerate() {
        elements.push(StageElement::Icon {
            id: format!("letter-{i}"),
            at: [375.0 + i as f32 * 165.0, 510.0, 0.0],
            size: 264.0,
            icon: String::new(),
            path: path.into(),
            view: 8.0,
            ink: None,
            tone: Tone::Plain,
        });
    }
    let mut s = stage(sc, elements)?;
    show(&mut s, sc, 0, &["equal"]);
    let weight = v.at("weight");
    hide(&mut s, sc, weight, &["equal"]);
    for i in 0..3 {
        let at = weight + seconds(i as f64 * 0.25);
        if i > 0 {
            hide(&mut s, sc, at, &[&format!("weight-{}", i - 1)]);
        }
        show(&mut s, sc, at, &[&format!("weight-{i}")]);
    }
    let at = v.at("interval between letters");
    hide(&mut s, sc, at, &["weight-2"]);
    show(&mut s, sc, at, &["interval"]);
    for i in 0..8 {
        let id = format!("letter-{i}");
        show(&mut s, sc, at, &[&id]);
        s.glide(
            sc,
            &format!("{id}.x"),
            v.at("too generous"),
            (i as f32 - 3.5) * 30.0,
            0.6,
        );
        s.glide(
            sc,
            &format!("{id}.x"),
            v.at("too narrow"),
            (i as f32 - 3.5) * -25.0,
            0.6,
        );
        s.glide(sc, &format!("{id}.x"), v.at("approved"), 0.0, 0.7);
    }
    Ok(())
}

fn pepsi(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut s = stage(
        sc,
        vec![
            shape(
                "white",
                [650.0, 490.0],
                Figure::Circle(270.0),
                Some(Tone::Plain),
                None,
                0.0,
            ),
            pigment(
                icon(
                    "red",
                    [650.0, 490.0],
                    724.0,
                    "M42 121A112 112 0 0 1 261 139C206 107 129 188 42 121Z",
                    Tone::Error,
                ),
                [227, 41, 52],
            ),
            pigment(
                icon(
                    "blue",
                    [650.0, 490.0],
                    724.0,
                    "M39 166C128 234 205 132 261 167A112 112 0 0 1 39 166Z",
                    Tone::Request,
                ),
                [0, 75, 147],
            ),
            text("name-1", 1420.0, 360.0, 66.0, "The refreshment"),
            text("name-2", 1420.0, 440.0, 66.0, "hypothesis."),
            note("recognition", 1420.0, 610.0, "Recognition: immediate."),
            text("rejected", 960.0, 490.0, 156.0, "Rejected."),
            text("dismissed", 960.0, 705.0, 60.0, "Designer dismissed."),
            note(
                "fiction",
                960.0,
                1010.0,
                "Speculative proposal · fictional personnel note",
            ),
        ],
    )?;
    show(&mut s, sc, 0, &["white", "name-1", "name-2", "fiction"]);
    show(&mut s, sc, v.at("red"), &["red"]);
    show(&mut s, sc, v.at("blue"), &["blue"]);
    show(&mut s, sc, v.at("Recognition"), &["recognition"]);
    let at = v.at("Disposition");
    hide(
        &mut s,
        sc,
        at,
        &["white", "red", "blue", "name-1", "name-2", "recognition"],
    );
    show(&mut s, sc, v.at("rejected"), &["rejected"]);
    show(&mut s, sc, v.at("Designer dismissed"), &["dismissed"]);
    Ok(())
}

fn alternatives(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut soft = shape(
        "soft",
        [960.0, 470.0],
        Figure::Rect([390.0, 520.0]),
        None,
        Some(Tone::Plain),
        95.0,
    );
    if let StageElement::Shape { corner, .. } = &mut soft {
        *corner = 85.0;
    }
    let mut elements = vec![
        text("studies", 960.0, 510.0, 140.0, "Further studies."),
        text("not-this", 960.0, 510.0, 140.0, "Not this."),
        shape(
            "master-a",
            [825.0, 480.0],
            Figure::Circle(250.0),
            Some(Tone::Error),
            None,
            0.0,
        ),
        shape(
            "master-b",
            [1095.0, 480.0],
            Figure::Circle(250.0),
            Some(Tone::Warning),
            None,
            0.0,
        ),
        shape(
            "target",
            [960.0, 480.0],
            Figure::Circle(260.0),
            None,
            Some(Tone::Error),
            70.0,
        ),
        shape(
            "target-dot",
            [960.0, 480.0],
            Figure::Circle(87.0),
            Some(Tone::Error),
            None,
            0.0,
        ),
        icon(
            "opera",
            [960.0, 480.0],
            650.0,
            "M150 20A95 130 0 1 1 149.99 20ZM150 65A50 85 0 1 0 150.01 65Z",
            Tone::Error,
        ),
        soft,
        icon(
            "gold",
            [960.0, 480.0],
            650.0,
            "M103.64745 60V240H196.35255V60ZM57.2949 0H242.7051V300H57.2949Z",
            Tone::Plain,
        ),
    ];
    for (id, name) in [
        ("convergence", "Convergence."),
        ("precision", "Precision."),
        ("operatic", "An operatic opening."),
        ("softer", "A softer boundary."),
        ("irrational", "An irrational accommodation."),
    ] {
        elements.push(text(id, 960.0, 930.0, 52.0, name));
    }
    let mut s = stage(sc, elements)?;
    let groups: [(&str, &[&str]); 5] = [
        ("convergence", &["master-a", "master-b", "convergence"]),
        ("Precision", &["target", "target-dot", "precision"]),
        ("operatic", &["opera", "operatic"]),
        ("softer boundary", &["soft", "softer"]),
        ("irrational", &["gold", "irrational"]),
    ];
    show(&mut s, sc, 0, &["studies"]);
    hide(&mut s, sc, v.at("convergence"), &["studies"]);
    for (i, (phrase, ids)) in groups.iter().enumerate() {
        let at = v.at(phrase);
        if i > 0 {
            hide(&mut s, sc, at, groups[i - 1].1);
        }
        show(&mut s, sc, at, ids);
    }
    // Contact-sheet rhythm: a rapid reprise, then a clean hold.
    let at = v.at("what the mark");
    hide(&mut s, sc, at, groups[4].1);
    for (i, (_, ids)) in groups.iter().enumerate() {
        show(&mut s, sc, at + seconds(i as f64 * 0.18), ids);
        hide(&mut s, sc, at + seconds((i + 1) as f64 * 0.18), ids);
    }
    show(&mut s, sc, at + seconds(0.9), &["not-this"]);
    Ok(())
}

fn finale(sc: &mut PlanBuilder, v: &Spoken<'_>) -> Result<()> {
    let mut elements = mark("o", 960.0, 465.0, 600.0);
    elements.extend([
        photo("cave"),
        photo("inferno"),
        circle("compass", 960.0, 500.0, 280.0),
        text("personnel", 960.0, 510.0, 90.0, "Personnel adjustment."),
        text("word", 960.0, 850.0, 52.0, "OpenCode"),
        note("dimensions", 960.0, 965.0, "4 × 5 / Curves: 0"),
        text("next", 960.0, 855.0, 56.0, "Room for what comes next."),
        note(
            "source-1",
            960.0,
            430.0,
            "Contemporary formal studies of the OpenCode mark.",
        ),
        note(
            "source-2",
            960.0,
            485.0,
            "Imagined plates. Fictional proposals and personnel notes.",
        ),
        note(
            "source-3",
            960.0,
            590.0,
            "Sources: Euclid · Vasari · Dante · Chauvet · Lascaux",
        ),
        note("source-4", 960.0, 645.0, "Exact artwork: opencode.ai/brand"),
    ]);
    let mut s = stage(sc, elements)?;
    show(&mut s, sc, v.at("caves"), &["cave"]);
    hide(&mut s, sc, v.at("compass"), &["cave"]);
    show(&mut s, sc, v.at("compass"), &["compass"]);
    hide(&mut s, sc, v.at("circles of Hell"), &["compass"]);
    show(&mut s, sc, v.at("circles of Hell"), &["inferno"]);
    hide(&mut s, sc, v.at("personnel adjustment"), &["inferno"]);
    show(&mut s, sc, v.at("personnel adjustment"), &["personnel"]);
    let at = v.at_any(&["returned to the opening", "return to the opening"]);
    hide(&mut s, sc, at, &["personnel"]);
    show(&mut s, sc, at, &["o-plane", "o-ring"]);
    for id in ["o-ring", "o-plane"] {
        s.channel(sc, &format!("{id}.scale"), 1.12);
        s.to(sc, &format!("{id}.scale"), at, 1.0, 1.8);
    }
    show(&mut s, sc, v.at("Four by five"), &["dimensions"]);
    show(&mut s, sc, v.at_any(&["Open Code", "OpenCode"]), &["word"]);
    let room = v.at("Room for");
    hide(&mut s, sc, room, &["word"]);
    show(&mut s, sc, room, &["next"]);
    let credits = sc.duration_nanos() - seconds(4.0);
    cut_all(
        &mut s,
        sc,
        credits,
        &["o-ring", "o-plane", "dimensions", "next"],
    );
    show(
        &mut s,
        sc,
        credits,
        &["source-1", "source-2", "source-3", "source-4"],
    );
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
    let mut segments = vec![];
    for id in IDS {
        let clip = narration.clip(id)?;
        let duration = clip.duration() + seconds(if id == "finale" { 6.0 } else { 1.0 });
        let mut sc = PlanBuilder::new(id, duration);
        let spoken = clip.place(&mut sc, seconds(0.35));
        match id {
            "opening" => opening(&mut sc, &spoken)?,
            "cave" => cave(&mut sc, &spoken)?,
            "circle" => circles(&mut sc, &spoken, false)?,
            "vesica" => circles(&mut sc, &spoken, true)?,
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
        if matches!(id, "cave" | "inferno" | "finale") {
            for name in ["cave", "inferno"] {
                if id == name || id == "finale" {
                    sc.media(footage::still(
                        name,
                        media_root.join(format!("assets/{name}.webp")),
                        0,
                        duration,
                    ));
                }
            }
        }
        let mut plan: ScenePlan = sc.finish()?;
        for media in &mut plan.media {
            if media.path.is_relative() {
                media.path = media_root.join(&media.path);
            }
        }
        segments.push(ReelSegmentPlan {
            plan,
            transition_nanos: 0,
            transition_style: ReelTransitionStyle::Dip,
            transition_focus: None,
            transition_wipe: None,
        });
    }
    let reel = ReelPlan {
        version: ReelPlan::VERSION,
        id: "the-shape-of-openness".into(),
        segments,
    };
    reel.validate()?;
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
