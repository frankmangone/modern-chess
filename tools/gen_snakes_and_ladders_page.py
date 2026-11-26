"""Writes crates/game/pages/snakes-and-ladders.json: the page text and the drawn board (the squares, ladders and snakes).

Run from the repository root; it rewrites the file it names.
"""
import json, math
LADDERS = {1:38, 4:14, 9:31, 21:42, 28:84, 36:44, 51:67, 71:91, 80:100}
SNAKES = {16:6, 47:26, 49:11, 56:53, 62:19, 64:60, 87:24, 93:73, 95:75, 98:78}
def square(n):
    row, col = divmod(n - 1, 10)
    if row % 2 == 1: col = 9 - col
    return col, row
def centre(n):  # in the art's box: x = col + 2, y = 10 - row (y down)
    c, r = square(n)
    return c + 2.0, 10.0 - r
f = lambda v: f"{v:.3f}".rstrip("0").rstrip(".")
parts = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 11" preserveAspectRatio="none">']
# the start pad
parts.append('<rect x="0.55" y="9.55" width="0.9" height="0.9" rx="0.12" fill="#cfe3b4" stroke="#6f8f4a" stroke-width="0.04"/>')
parts.append('<text x="1" y="10.08" font-size="0.2" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#4a6a2c">START</text>')
for n in range(1, 101):
    c, r = square(n)
    x, y = c + 1.5, 9.5 - r
    fill = "#f4e7c6" if (c + r) % 2 == 0 else "#e4cf9f"
    parts.append(f'<rect x="{f(x)}" y="{f(y)}" width="1" height="1" fill="{fill}" stroke="#b79a62" stroke-width="0.02"/>')
    parts.append(f'<text x="{f(x+0.06)}" y="{f(y+0.24)}" font-size="0.22" font-family="sans-serif" font-weight="600" fill="#8a6d3b">{n}</text>')
parts.append('<rect x="1.5" y="-0.5" width="0" height="0"/>')
# ladders
for a, b in LADDERS.items():
    (x1, y1), (x2, y2) = centre(a), centre(b)
    dx, dy = x2 - x1, y2 - y1
    L = math.hypot(dx, dy); ux, uy = dx / L, dy / L; px, py = -uy * 0.13, ux * 0.13
    parts.append(f'<g stroke="#8a5a2b" stroke-linecap="round" opacity="0.92">')
    parts.append(f'<line x1="{f(x1+px)}" y1="{f(y1+py)}" x2="{f(x2+px)}" y2="{f(y2+py)}" stroke-width="0.07"/>')
    parts.append(f'<line x1="{f(x1-px)}" y1="{f(y1-py)}" x2="{f(x2-px)}" y2="{f(y2-py)}" stroke-width="0.07"/>')
    rungs = max(2, int(L / 0.42))
    for i in range(1, rungs):
        t = i / rungs
        cx, cy = x1 + dx * t, y1 + dy * t
        parts.append(f'<line x1="{f(cx+px)}" y1="{f(cy+py)}" x2="{f(cx-px)}" y2="{f(cy-py)}" stroke-width="0.05"/>')
    parts.append('</g>')
# snakes: head at the higher square, tail where it lands
colours = ["#3f9a4a", "#7a4fb0", "#c2543a", "#2f8f9a", "#b8913a"]
for i, (a, b) in enumerate(sorted(SNAKES.items())):
    (x1, y1), (x2, y2) = centre(a), centre(b)
    dx, dy = x2 - x1, y2 - y1
    L = math.hypot(dx, dy); nx, ny = -dy / L, dx / L
    bend = min(1.0, 0.35 * L / 3 + 0.2)
    c1 = (x1 + dx * 0.3 + nx * bend, y1 + dy * 0.3 + ny * bend)
    c2 = (x1 + dx * 0.7 - nx * bend, y1 + dy * 0.7 - ny * bend)
    col = colours[i % len(colours)]
    parts.append(f'<path d="M{f(x1)} {f(y1)} C{f(c1[0])} {f(c1[1])} {f(c2[0])} {f(c2[1])} {f(x2)} {f(y2)}" fill="none" stroke="{col}" stroke-width="0.17" stroke-linecap="round" opacity="0.9"/>')
    parts.append(f'<circle cx="{f(x1)}" cy="{f(y1)}" r="0.16" fill="{col}"/>')
    parts.append(f'<circle cx="{f(x1-0.05)}" cy="{f(y1-0.04)}" r="0.035" fill="#fff"/><circle cx="{f(x1+0.05)}" cy="{f(y1-0.04)}" r="0.035" fill="#fff"/>')
parts.append('</svg>')
art = "".join(parts)
token = lambda fill, edge: f'<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="42" fill="{fill}" stroke="{edge}" stroke-width="8"/><ellipse cx="38" cy="36" rx="12" ry="7" fill="#ffffff55"/></svg>'
pip = '<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="8" y="8" width="84" height="84" rx="16" fill="#fbf6ea" stroke="#5a4326" stroke-width="6"/><circle cx="50" cy="50" r="16" fill="#3b2a14"/></svg>'
page = {
    "slug": "snakes-and-ladders",
    "name": "Snakes and Ladders",
    "tagline": "Race up the board on the roll of a die; ladders lift you, snakes drop you.",
    "description": "A race game for two on a numbered board of a hundred squares. Roll a die and move that many squares. Climb a ladder to jump ahead, slide down a snake to fall back, and land exactly on 100 to win.",
    "tags": ["classic", "two players", "dice", "luck"],
    "accent": "#3f9a4a",
    "skin": {
        "captured_panel": False,
        "board": {"background": "#e9d9ae", "art": art},
        "pieces": {
            "RED_TOKEN": {"svg": token("#c0392b", "#5e1610")},
            "BLUE_TOKEN": {"svg": token("#2f5fa8", "#10254a")},
            "PIP": {"svg": pip},
        },
    },
    "sections": [
        {"type": "how_to_play", "markdown": (
            "Snakes and Ladders is a race along a numbered board of **100 squares**. Both players start off the board, and **Red** rolls first.\n\n"
            "- **Roll.** On your turn, roll the die and move your token that many squares, following the numbers: left to right on the bottom row, then right to left on the next, and so on up the board.\n"
            "- **Ladders.** If you end on the foot of a ladder, climb it to the square at its top.\n"
            "- **Snakes.** If you end on a snake's head, slide down to the square at its tail.\n"
            "- **Sharing.** Both tokens may stand on the same square. Nothing is captured.\n"
            "- **Winning.** The first token to land **exactly** on square 100 wins. If your roll would take you past 100, you stay where you are and the turn passes.\n\n"
            "Nothing is chosen in this game: every move is the roll, so luck decides it.")},
        {"type": "preview"},
        {"type": "open_lobbies"},
    ],
    "featured": False,
    "sort_order": 20,
}
json.dump(page, open("crates/game/pages/snakes-and-ladders.json", "w"), indent=2)
open("crates/game/pages/snakes-and-ladders.json", "a").write("\n")
print(len(art), "bytes of art")
