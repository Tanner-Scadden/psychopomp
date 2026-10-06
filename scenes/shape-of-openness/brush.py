# Giotto's circle as one brush stroke in red: bristles that carry and lose ink,
# a touch-down blob, a dry-brush lift, and paper grain. Renders a straight-alpha
# PNG sequence the Scene Program plays as Stage footage at 60 fps:
#
#   output/shape-of-openness/assets/giotto/000.png ... 059.png
#
#   uv run --with numpy --with scipy --with pillow python scenes/shape-of-openness/brush.py
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage as ndi

OUT = Path(__file__).resolve().parents[2] / "output/shape-of-openness/assets/giotto"
SIZE, RADIUS, WIDTH = 720, 300.0, 33.0
START, SWEEP = np.radians(-14.0), np.radians(374.0)  # clockwise from just left of twelve
FRAMES, DRAWING = 60, 55
RED = np.array([0.69, 0.1, 0.085], np.float32)
rng = np.random.default_rng(1305)

ys, xs = np.mgrid[0:SIZE, 0:SIZE].astype(np.float32) - SIZE / 2 + 0.5
r = np.hypot(xs, ys)
# Clockwise angle from twelve o'clock, relative to the stroke's start.
angle = np.mod(np.arctan2(xs, -ys) - START, 2 * np.pi)
grain = 1 + 0.16 * ndi.gaussian_filter(rng.standard_normal((SIZE, SIZE)), 1.1) / 0.18
grain *= 1 + 0.08 * ndi.gaussian_filter(rng.standard_normal((SIZE, SIZE)), 6) / 0.05


def pressure(u):
    landing = 0.62 + 0.38 * np.clip(u / 0.035, 0, 1) ** 0.6 + 0.22 * np.exp(-((u - 0.012) / 0.012) ** 2)
    lift = 1 - 0.8 * np.clip((u - 0.88) / 0.12, 0, 1) ** 1.6
    return landing * lift * (1 + 0.05 * np.sin(u * 17.0 + 0.6) + 0.03 * np.sin(u * 41.0))


bristles = [
    {
        "offset": rng.uniform(-0.5, 0.5),
        "wander": rng.uniform(0.3, 1.1),
        "phase": rng.uniform(0, 2 * np.pi),
        "rate": rng.uniform(3.0, 9.0),
        "width": rng.uniform(0.9, 1.8),
        "load": rng.uniform(0.75, 1.0),
        "dry": rng.uniform(0.7, 1.15),
    }
    for _ in range(54)
]


def stroke(theta):
    """Pigment alpha where the stroke passes at `theta` radians along its sweep."""
    u = theta / SWEEP
    radius = RADIUS * (1 + 0.004 * np.sin(2 * theta + 0.7) + 0.003 * np.sin(3 * theta + 2.1))
    across = r - radius
    width = WIDTH * pressure(u)
    total = np.zeros_like(r)
    for b in bristles:
        edge = abs(b["offset"]) * 2
        dry = np.clip((u - (b["dry"] - 0.25 * edge)) / 0.18, 0, 1)
        ink = b["load"] * (1 - dry) * (1 - 0.25 * u)
        center = b["offset"] * width + b["wander"] * np.sin(theta * b["rate"] + b["phase"])
        total += ink * np.exp(-(((across - center) / b["width"]) ** 2))
    body = np.exp(-((np.abs(across) / (0.5 * width + 0.6)) ** 6))
    total = total * 0.42 + body * 0.55 * (1 - np.clip((u - 0.9) / 0.1, 0, 1))
    return np.clip(1 - np.exp(-1.7 * total * grain), 0, 1)


first = stroke(angle) * np.clip(angle / np.radians(0.8), 0, 1)
overlap = np.where(angle < SWEEP - 2 * np.pi, stroke(angle + 2 * np.pi), 0.0)
# Where the loaded brush first lands: a rounded, slightly wider blob.
sx, sy = np.sin(START) * RADIUS, -np.cos(START) * RADIUS
landing = np.exp(-(((xs - sx) ** 2 + (ys - sy) ** 2) / (0.56 * WIDTH) ** 2) ** 2.5)
landing = np.clip(landing * grain * 1.05, 0, 0.97)

OUT.mkdir(parents=True, exist_ok=True)
for frame in range(FRAMES):
    t = min(frame / DRAWING, 1.0)
    tip = SWEEP * (1 - (1 - t) ** 1.85)
    shown_first = np.clip((tip - angle) / np.radians(1.2), 0, 1)
    shown_overlap = np.clip((tip - angle - 2 * np.pi) / np.radians(1.2), 0, 1)
    alpha = 1 - (1 - first * shown_first) * (1 - overlap * shown_overlap) * (1 - landing)
    if frame < DRAWING:
        # The loaded brush head, wetter and darker than the drying stroke.
        head = START + tip
        cx, cy = np.sin(head) * RADIUS, -np.cos(head) * RADIUS
        bead = np.exp(-(((xs - cx) ** 2 + (ys - cy) ** 2) / (0.42 * WIDTH * pressure(tip / SWEEP)) ** 2) ** 2)
        alpha = np.maximum(alpha, 0.9 * bead)
    rgb = RED * (1 - 0.28 * alpha[..., None] ** 2)
    data = np.dstack([np.broadcast_to(rgb, (SIZE, SIZE, 3)) if rgb.ndim == 1 else rgb, alpha])
    Image.fromarray((np.clip(data, 0, 1) * 255 + 0.5).astype(np.uint8)).save(OUT / f"{frame:03d}.png", optimize=True)
print(f"{FRAMES} frames → {OUT}")
