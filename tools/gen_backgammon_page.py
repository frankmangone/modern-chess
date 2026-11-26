"""Writes crates/game/pages/backgammon.json: the page text, the board art and the piece faces.

Run from the repository root; it rewrites the file it names.
"""
import json
f = lambda v: f"{v:.3f}".rstrip("0").rstrip(".")
# Box: 15 wide, 9 tall. A place at map [x, y] sits at art (x + 1, 8 - y).
FRAME, FELT, BAR = "#4a2c14", "#2f6b4a", "#6b4426"
TRI_A, TRI_B = "#e8d6a6", "#8c3b2a"
LEFT, RIGHT = 0.5, 7.5          # left edges of the two halves
parts = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 15 9" preserveAspectRatio="none">']
parts.append(f'<rect x="0" y="0" width="15" height="9" fill="{FRAME}"/>')
parts.append(f'<rect x="0.5" y="0.5" width="6" height="8" fill="{FELT}"/>')
parts.append(f'<rect x="7.5" y="0.5" width="6" height="8" fill="{FELT}"/>')
parts.append(f'<rect x="6.5" y="0.5" width="1" height="8" fill="{BAR}"/>')
parts.append(f'<text x="7" y="4.62" font-size="0.3" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#d9bf8f" letter-spacing="0.08">BAR</text>')
# Trays: black's above, white's below.
for y0, name in ((0.55, "BLACK"), (7.55, "WHITE")):
    parts.append(f'<rect x="13.55" y="{y0}" width="0.9" height="0.9" rx="0.1" fill="#3a2410" stroke="#d9bf8f" stroke-width="0.03"/>')
parts.append(f'<text x="14" y="1.78" font-size="0.2" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#d9bf8f">BLACK</text>')
parts.append(f'<text x="14" y="7.4" font-size="0.2" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#d9bf8f">WHITE</text>')


def column(point):
    if point >= 13:
        return (point - 13) if point <= 18 else (point - 19 + 7)
    return (12 - point) if point >= 7 else (6 - point + 7)


for p in range(1, 25):
    x = column(p)
    cx = x + 1
    left = cx - 0.5
    top = p >= 13
    fill = TRI_A if (p % 2 == 0) == top else TRI_B
    if top:
        pts = f"{f(left)},0.5 {f(left + 1)},0.5 {f(cx)},4"
        num_y = 0.36
    else:
        pts = f"{f(left)},8.5 {f(left + 1)},8.5 {f(cx)},5"
        num_y = 8.82
    parts.append(f'<polygon points="{pts}" fill="{fill}" stroke="#00000033" stroke-width="0.02"/>')
    parts.append(f'<text x="{f(cx)}" y="{num_y}" font-size="0.26" font-family="sans-serif" font-weight="600" text-anchor="middle" fill="#d9bf8f">{p}</text>')
parts.append('</svg>')
art = "".join(parts)


def disc(fill, edge, shine):
    return (f'<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="44" fill="{fill}" stroke="{edge}" stroke-width="7"/>'
            f'<circle cx="50" cy="50" r="30" fill="none" stroke="{edge}" stroke-width="3" opacity="0.5"/>'
            f'<ellipse cx="36" cy="34" rx="13" ry="7" fill="{shine}"/></svg>')


PIPS = {
    1: [(50, 50)], 2: [(28, 28), (72, 72)], 3: [(28, 28), (50, 50), (72, 72)],
    4: [(28, 28), (72, 28), (28, 72), (72, 72)], 5: [(28, 28), (72, 28), (50, 50), (28, 72), (72, 72)],
    6: [(28, 26), (72, 26), (28, 50), (72, 50), (28, 74), (72, 74)],
}


def die(n):
    dots = "".join(f'<circle cx="{x}" cy="{y}" r="9" fill="#3b2a14"/>' for x, y in PIPS[n])
    return ('<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="8" y="8" width="84" height="84" rx="16" '
            f'fill="#fbf6ea" stroke="#5a4326" stroke-width="6"/>{dots}</svg>')


pieces = {
    "WHITE_CHECKER": {"svg": disc("#f3ecd8", "#8c7a52", "#ffffffaa")},
    "BLACK_CHECKER": {"svg": disc("#2b2622", "#0c0a09", "#ffffff33")},
}
for n in range(1, 7):
    pieces[f"D{n}"] = {"svg": die(n)}

how = (
    "Backgammon is a race with hitting. Each side has **fifteen checkers** and moves them all round the board and "
    "off it. **White** moves from point 24 down to point 1 and bears off from the home board (points 1 to 6); "
    "**Black** moves the other way, from point 1 up to 24, and bears off from 19 to 24. White rolls first.\n\n"
    "- **Roll.** Press *Roll* to throw two dice. Each die is a move of one checker that many points. "
    "A **double** is played four times.\n"
    "- **Moving.** Click the point a checker should go to. Several dice can reach the same point, and then the "
    "buttons ask which one to use. A point holding two or more enemy checkers is closed to you. A point with a single "
    "enemy checker (a *blot*) can be landed on: the blot is **hit** and goes to its owner's bar.\n"
    "- **The bar.** While you have a checker on your bar you play nothing else: enter it on the point that the die shows "
    "in the enemy's home board (a 1 enters on White's point 24, or Black's point 1, and so on). "
    "If you cannot, the turn passes.\n"
    "- **Bearing off.** When all your checkers are in your home board, click your tray to take one off with a die: "
    "the exact number, or a larger one when no checker stands farther back.\n"
    "- **Winning.** The first side to bear off all fifteen checkers wins.\n\n"
    "Left out: the doubling cube, and the extra points for gammons. A game is one win. In real Backgammon both dice must be "
    "played when they can. This game does not check that: it lets you play a die that leaves the other unplayable, "
    "and that die is lost, so look before you move."
)
page = {
    "slug": "backgammon",
    "name": "Backgammon",
    "tagline": "Race your fifteen checkers home on the roll of the dice, and hit the blots on the way.",
    "description": "The oldest race game still played. Two dice move your checkers round twenty-four points to your home board, where you bear them off. Land on a lone enemy checker and it goes back to the bar; a point with two of your checkers is a wall.",
    "tags": ["classic", "two players", "dice", "strategy"],
    "accent": "#8c3b2a",
    "skin": {
        "captured_panel": False,
        "hint": "Click where a checker should go.",
        "board": {"background": FRAME, "art": art},
        "pieces": pieces,
    },
    "sections": [
        {"type": "how_to_play", "markdown": how},
        {"type": "preview"},
        {"type": "open_lobbies"},
    ],
    "featured": False,
    "sort_order": 21,
}
out = "crates/game/pages/backgammon.json"
json.dump(page, open(out, "w"), indent=2)
open(out, "a").write("\n")
print(len(art), "bytes of art")
