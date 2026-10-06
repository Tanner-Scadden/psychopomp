# Derives the cave sequence's layers from two generated images, so the stencil
# is exactly the absence of the hand that made it:
#
#   output/shape-of-openness/assets/wall-gen.png  2048x1152 bare limestone wall
#   output/shape-of-openness/assets/hand-gen.png  1024x1536 hand on chroma green
#
# Writes registered layers in the wall's canvas to output/shape-of-openness/assets/
# and the small geometry the Scene Program needs (layer boxes, the trace polygon,
# the absence's path) to scenes/shape-of-openness/stencil.json.
#
#   uv run --with numpy --with scipy --with pillow --with scikit-image \
#     python scenes/shape-of-openness/stencil.py
import json
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage as ndi
from skimage import measure

SCENE = Path(__file__).resolve().parent
ASSETS = SCENE.parents[1] / "output/shape-of-openness/assets"

# Hand placement in the canvas: scale, top edge, and the center of its width.
SCALE, TOP, CENTER_X = 0.6, 245, 1010
# Red ochre: how the wall's own color is multiplied where pigment lands, and
# the body color dense pigment tends toward.
TINT = np.array([0.6, 0.19, 0.1], np.float32)
OCHRE = np.array([0.36, 0.08, 0.035], np.float32)
rng = np.random.default_rng(1994)


def load(name):
    return np.asarray(Image.open(ASSETS / name).convert("RGB"), np.float32) / 255


def save(name, rgb, alpha=None, scale=1.0, box=None):
    """Write straight-alpha 8-bit layers, cropped to `box` (x, y, w, h)."""
    if box is not None:
        x, y, w, h = box
        rgb = rgb[y : y + h, x : x + w]
        alpha = None if alpha is None else alpha[y : y + h, x : x + w]
    data = rgb if alpha is None else np.dstack([rgb, alpha])
    image = Image.fromarray((np.clip(data, 0, 1) * 255 + 0.5).astype(np.uint8))
    if scale != 1.0:
        image = image.resize((round(image.width * scale), round(image.height * scale)), Image.LANCZOS)
    if name.endswith(".webp"):
        image.save(ASSETS / name, quality=93, method=6)
    else:
        image.save(ASSETS / name, optimize=True)


def blur(field, sigma):
    return ndi.gaussian_filter(field, sigma, mode="nearest")


def smooth_noise(shape, sigma):
    noise = blur(rng.standard_normal(shape).astype(np.float32), sigma)
    return noise / (noise.std() + 1e-6)


def smoothstep(edge0, edge1, x):
    t = np.clip((x - edge0) / (edge1 - edge0), 0, 1)
    return t * t * (3 - 2 * t)


def bounds(alpha, pad, limit):
    rows, cols = np.nonzero(alpha > 0.004)
    x0, y0 = max(cols.min() - pad, 0), max(rows.min() - pad, 0)
    x1, y1 = min(cols.max() + pad + 1, limit[1]), min(rows.max() + pad + 1, limit[0])
    return [int(x0), int(y0), int(x1 - x0), int(y1 - y0)]


wall = load("wall-gen.png")
H, W = wall.shape[:2]

# Key the hand: a pixel is a linear mix of skin and the green screen, so its
# green dominance gives alpha, and unmixing removes the green fringe.
source = load("hand-gen.png")
screen = np.median(np.concatenate([source[:40, :40].reshape(-1, 3), source[:40, -40:].reshape(-1, 3)]), axis=0)
dominance = source[..., 1] - np.maximum(source[..., 0], source[..., 2])
skin = -0.2
background = screen[1] - max(screen[0], screen[2])
alpha = np.clip((background - dominance) / (background - skin), 0, 1)
alpha = np.where(alpha < 0.03, 0, alpha)
alpha = np.maximum(alpha, ndi.binary_fill_holes(alpha > 0.5).astype(np.float32))
alpha = np.minimum(alpha, blur(ndi.grey_erosion(alpha, size=3), 0.6) + 0.02)
alpha = np.clip(alpha, 0, 1)
safe = np.maximum(alpha, 1e-3)[..., None]
color = np.clip((source - (1 - alpha[..., None]) * screen) / safe, 0, 1)
color[..., 1] = np.minimum(color[..., 1], np.maximum(color[..., 0], color[..., 2]) * 0.95 + 0.03)

# Into the canvas, resampled premultiplied so edges carry no fringe.
size = (round(source.shape[1] * SCALE), round(source.shape[0] * SCALE))
left = round(CENTER_X - 490 * SCALE)
premultiplied = Image.fromarray(np.uint8(np.dstack([color * alpha[..., None], alpha]) * 255 + 0.5), "RGBA")
premultiplied = premultiplied.resize(size, Image.LANCZOS)
placed = np.zeros((H, W, 4), np.float32)
patch = np.asarray(premultiplied, np.float32) / 255
y1, x1 = min(TOP + size[1], H), min(left + size[0], W)
placed[TOP:y1, left:x1] = patch[: y1 - TOP, : x1 - left]
M = placed[..., 3]
hand = placed[..., :3] / np.maximum(M, 1e-3)[..., None]

# Grade the skin into the lamp light: warmer, a little darker, brighter toward
# the lamp at lower left.
ys, xs = np.mgrid[0:H, 0:W].astype(np.float32)
lamp = 1.0 + 0.16 * ((ys - 700) / 600 - (xs - 1010) / 700)
luma = (hand @ np.array([0.2126, 0.7152, 0.0722], np.float32))[..., None]
hand = luma + (hand - luma) * 0.82
hand = np.clip(hand * np.array([0.97, 0.88, 0.77], np.float32) * (0.86 * lamp)[..., None], 0, 1)

# Where the wrist begins, below which the arm shields the wall from spray.
wrist = TOP + 1150 * SCALE
reach = 1 - 0.8 * smoothstep(wrist - 60, wrist + 230, ys)

# Spray: dense against the edge, a softer cloud beyond, heaviest between the
# fingers where both sides overlap. Three cumulative stages, one per breath.
hard = (M > 0.5).astype(np.float32)
outside = 1 - blur(hard, 1.2)
grain = (
    1
    + 0.2 * smooth_noise((H, W), 110)
    + 0.3 * smooth_noise((H, W), 38)
    + 0.22 * smooth_noise((H, W), 11)
    + 0.12 * smooth_noise((H, W), 2.5)
)


def stage(spread):
    near, mid, far = (2 * blur(hard, s * spread) for s in (13, 58, 165))
    cloud = 0.62 * near + 0.5 * mid + 0.36 * far
    droplets = (rng.random((H, W)) < 0.05 * far).astype(np.float32)
    cloud = cloud * grain + 0.55 * blur(droplets, 0.8) * far
    return np.clip(cloud * outside * reach, 0, 0.97)


stages = [stage(0.55), stage(0.8), stage(1.0)]
pigment = wall * TINT
pigment = pigment * 0.66 + OCHRE * 0.34
# Each breath is an opaque plate composited here in sRGB, so the renderer's
# linear-light crossfades end exactly on the intended pigment.
final = np.zeros((H, W), np.float32)
boxes = {}
for index, cumulative in enumerate(stages, 1):
    final = np.maximum(cumulative, final)
    boxes[f"stencil-{index}"] = [0, 0, W, H]
    save(f"stencil-{index}.webp", wall * (1 - final[..., None]) + pigment * final[..., None])

# The back of the hand takes pigment too, heavier toward its edges.
coat = M * np.clip(grain * (0.14 + 0.9 * (1 - blur(hard, 20))), 0, 0.6) * reach
coated = hand * (1 - coat[..., None]) + (hand * TINT * 0.8 + OCHRE * 0.2) * coat[..., None]
hand_box = bounds(M, 6, (H, W))
boxes["hand"] = hand_box
boxes["hand-ochre"] = hand_box
save("hand.png", hand, M, box=hand_box)
save("hand-ochre.png", coated, M, box=hand_box)

# A tight contact shadow up and to the right, away from the lamp; the plan
# shifts and softens it as the hand lifts away.
shadow = blur(hard, 5) * 0.78
shadow_box = bounds(shadow, 24, (H, W))
boxes["hand-shadow"] = shadow_box
save("hand-shadow.png", np.zeros((H, W, 3), np.float32), shadow, scale=0.5, box=shadow_box)

# Airborne pigment for each breath: a broad, faint cloud.
mist = np.clip(blur(final, 42) * 0.9 * reach, 0, 1)
mist_box = bounds(mist, 0, (H, W))
boxes["mist"] = mist_box
save("mist.png", np.broadcast_to(OCHRE * 1.25, (H, W, 3)), mist, scale=0.25, box=mist_box)

save("wall.webp", wall)
stencilled = wall * (1 - final[..., None]) + pigment * final[..., None]
shaded = stencilled * (1 - 0.5 * np.roll(np.roll(shadow, -5, axis=0), 5, axis=1)[..., None])
contact = shaded * (1 - M[..., None]) + coated * M[..., None]
preview = np.concatenate([contact, stencilled], axis=1)
Image.fromarray(np.uint8(np.clip(preview, 0, 1) * 255)).resize((2048, 576), Image.LANCZOS).save(
    SCENE.parents[1] / "output/shape-of-openness/stencil-preview.jpg", quality=90
)

# The absence: its outline (closed along the canvas bottom), simplified to a
# Stage polygon for the trace, and in detail as filled path data.
padded = np.pad(blur(hard, 1.0), 1)
contour = max(measure.find_contours(padded, 0.5), key=len)[:, ::-1] - 1
contour[:, 1] = np.minimum(contour[:, 1], H - 0.5)
tolerance = 2.0
while len(outline := measure.approximate_polygon(contour, tolerance)[:-1]) > 64:
    tolerance *= 1.15
start = int(np.argmax(outline[:, 1] * 4 - outline[:, 0]))  # bottom left first
outline = np.roll(outline, -start, axis=0)
if outline[1][1] > outline[-1][1]:
    outline = np.concatenate([outline[:1], outline[1:][::-1]])
detail = measure.approximate_polygon(contour, 0.4)
side = int(max(hand_box[2], hand_box[3]))
origin = [hand_box[0] + hand_box[2] / 2 - side / 2, hand_box[1] + hand_box[3] / 2 - side / 2]
path = "M" + "L".join(f"{x - origin[0]:.1f} {y - origin[1]:.1f}" for x, y in detail) + "Z"

json.dump(
    {
        "canvas": [W, H],
        "layers": boxes,
        "outline": [[round(float(x), 1), round(float(y), 1)] for x, y in outline],
        "absence": {"box": [round(origin[0], 1), round(origin[1], 1), side], "path": path},
    },
    open(SCENE / "stencil.json", "w"),
    separators=(",", ":"),
)
print(json.dumps({"layers": boxes, "outline": len(outline), "path": len(path)}))
