"""Writes crates/engine/specs/backgammon.json: the points, the links, the zones and the per-die actions.

Run from the repository root; it rewrites the file it names.
"""
import json, sys

# Points are numbered as White sees them: White moves from 24 down to 1 and bears off past 1;
# Black the other way. Home boards: White 1-6, Black 19-24.
SIDES = {
    "WHITE": {"code": "WHITE_CHECKER", "label": "w", "other": "BLACK"},
    "BLACK": {"code": "BLACK_CHECKER", "label": "b", "other": "WHITE"},
}
START = {
    "WHITE_CHECKER": {"24": 2, "13": 5, "8": 3, "6": 5},
    "BLACK_CHECKER": {"1": 2, "12": 5, "17": 3, "19": 5},
}


def at_of(point):
    # Left half is x 0..5, the bar is x 6, the right half is x 7..12; the top row is y 1, the bottom y 0.
    if point >= 13:
        x = (point - 13) if point <= 18 else (point - 19 + 7)
        return [x, 7]
    x = (12 - point) if point >= 7 else (6 - point + 7)
    return [x, 0]


places = []
for p in range(1, 25):
    place = {"name": str(p), "at": at_of(p), "capacity": 15}
    start = {}
    for code, pts in START.items():
        if str(p) in pts:
            start[code] = pts[str(p)]
    if start:
        place["start"] = start
    places.append(place)

places.append({"name": "OFFW", "at": [13, 0], "capacity": 15})
places.append({"name": "OFFB", "at": [13, 7], "capacity": 15})

links = []
for n in range(1, 7):
    for p in range(1, 25):
        if p - n >= 1:
            links.append({"from": str(p), "to": str(p - n), "label": f"w{n}"})
        if p + n <= 24:
            links.append({"from": str(p), "to": str(p + n), "label": f"b{n}"})


def zone(name, white, black):
    return {"name": name, "places": {"WHITE": [str(x) for x in white], "BLACK": [str(x) for x in black]}}


zones = [zone("HOME", range(1, 7), range(19, 25))]
zones.append({"name": "TRAY", "places": {"WHITE": ["OFFW"], "BLACK": ["OFFB"]}})
zones.append({"name": "SAFE", "places": {"WHITE": ["OFFW"] + [str(x) for x in range(1, 7)],
                                        "BLACK": ["OFFB"] + [str(x) for x in range(19, 25)]}})
for k in range(1, 7):
    zones.append(zone(f"P{k}", [k], [25 - k]))
for k in range(1, 6):
    zones.append(zone(f"FARTHER{k}", range(k + 1, 25), range(1, 25 - k)))
for n in range(1, 7):
    zones.append(zone(f"ENTRY{n}", [25 - n], [n]))


def bag(name, of="MOVER"):
    return {"bag": name, "of": of}


def count(region, where=None, **kw):
    t = {"test": "COUNT", "in": region}
    if where is not None:
        t["where"] = where
    t.update(kw)
    return t


def at_bag(name, of="MOVER"):
    return {"region": "AT", "at": bag(name, of)}


def at_place(place):
    return {"region": "AT", "at": {"place": place}}


def has_die(n):
    return count(at_bag("DICE"), {"codes": [f"D{n}"]}, min=1)


def spend(n):
    return {"effect": "REMOVE", "at": bag("DICE"), "what": {"codes": [f"D{n}"]}, "count": 1}


STAY = {"effect": "STAGE", "name": "PLAY"}
def bar_of(side):
    # One shared bag for each side's bar, so the checkers on it stay nobody's like the ones on the
    # points (a piece that enters a player's own bag becomes theirs, and a point would list the two).
    return {"bag": f"{side.title()} bar"}


def no_bar(side):
    return count({"region": "AT", "at": bar_of(side)}, None, min=0, max=0)


def hit(side, place="TARGET"):
    """A lone enemy checker on the point goes to the bar."""
    other = SIDES[side]["other"]
    oc = SIDES[other]["code"]
    return {
        "if": count(at_place(place), {"codes": [oc]}, min=1),
        "then": [{"effect": "MOVE", "from": {"place": place}, "to": bar_of(other),
                  "what": {"codes": [oc]}, "count": 1}],
    }


def open_point(side, place="TARGET"):
    oc = SIDES[SIDES[side]["other"]]["code"]
    return count(at_place(place), {"codes": [oc]}, min=0, max=1)


def move_side(side, n):
    s = SIDES[side]
    code = s["code"]
    behind = {"region": "AROUND", "of": {"place": "TARGET"}, "through": f"{SIDES[s['other']]['label']}{n}"}
    return [
        has_die(n),
        no_bar(side),
        {"choose": "PLACE", "in": {"region": "BOARD"}},
        {"test": "SOME", "in": behind, "that": count(at_place("EACH"), {"codes": [code]}, min=1)},
        open_point(side),
        hit(side),
        {"effect": "FOR_EACH", "in": behind, "do": [
            {"effect": "MOVE", "from": {"place": "EACH"}, "to": {"place": "TARGET"},
             "what": {"codes": [code]}, "count": 1}]},
        spend(n),
        STAY,
    ]


def enter_side(side, n):
    s = SIDES[side]
    code = s["code"]
    return [
        has_die(n),
        {"choose": "PLACE", "in": {"region": "BOARD"}},
        {"test": "IN_ZONE", "at": {"place": "TARGET"}, "zone": f"ENTRY{n}"},
        open_point(side),
        hit(side),
        {"effect": "MOVE", "from": bar_of(side), "to": {"place": "TARGET"},
         "what": {"codes": [code]}, "count": 1},
        spend(n),
        STAY,
    ]


def off_side(side, n):
    s = SIDES[side]
    code = s["code"]
    outside = {"test": "SOME", "in": {"region": "BOARD"}, "that": {"test": "ALL", "of": [
        {"test": "NOT", "of": {"test": "IN_ZONE", "at": {"place": "EACH"}, "zone": "SAFE"}},
        count(at_place("EACH"), {"codes": [code]}, min=1)]}}
    # A checker that may go off with this die: on the point of its number, or on a lower one with
    # nothing farther back.
    ways = [{"test": "IN_ZONE", "at": {"place": "EACH"}, "zone": f"P{n}"}]
    for k in range(1, n):
        farther = {"test": "SOME", "in": {"region": "BOARD"}, "that": {"test": "ALL", "of": [
            {"test": "IN_ZONE", "at": {"place": "EACH"}, "zone": f"FARTHER{k}"},
            count(at_place("EACH"), {"codes": [code]}, min=1)]}}
        ways.append({"test": "ALL", "of": [
            {"test": "IN_ZONE", "at": {"place": "EACH"}, "zone": f"P{k}"},
            {"test": "NOT", "of": farther}]})
    goes = {"test": "ALL", "of": [count(at_place("EACH"), {"codes": [code]}, min=1),
                                  {"test": "ANY", "of": ways}]}
    return [
        has_die(n),
        no_bar(side),
        {"test": "NOT", "of": outside},
        {"choose": "PLACE", "in": {"region": "BOARD"}},
        {"test": "IN_ZONE", "at": {"place": "TARGET"}, "zone": "TRAY"},
        {"test": "SOME", "in": {"region": "BOARD"}, "that": goes},
        {"effect": "FOR_EACH", "in": {"region": "BOARD"}, "do": [
            {"if": goes, "then": [
                {"effect": "MOVE", "from": {"place": "EACH"}, "to": {"place": "TARGET"},
                 "what": {"codes": [code]}, "count": 1}]}]},
        spend(n),
        STAY,
    ]


def per_side(build, n):
    return [{"if": {"test": "PLAYER", "is": ["WHITE"]}, "then": build("WHITE", n), "else": build("BLACK", n)}]


faces = [{f"D{k}": 1} for k in range(1, 7)]
roll = {"name": "ROLL", "do": [
    {"effect": "REMOVE", "at": bag("DICE"), "count": "ALL"},
    {"effect": "REMOVE", "at": bag("DICE", "ENEMY"), "count": "ALL"},
    {"effect": "ROLL", "at": bag("DICE"), "dice": 2, "faces": faces},
] + [
    {"if": count(at_bag("DICE"), {"codes": [f"D{k}"]}, min=2),
     "then": [{"effect": "PLACE", "at": bag("DICE"), "code": f"D{k}", "count": 2}]}
    for k in range(1, 7)
] + [STAY]}

actions = [roll]
for n in range(1, 7):
    actions.append({"name": f"MOVE_{n}", "stage": "PLAY", "do": per_side(move_side, n)})
for n in range(1, 7):
    actions.append({"name": f"ENTER_{n}", "stage": "PLAY", "do": per_side(enter_side, n)})
for n in range(1, 7):
    actions.append({"name": f"OFF_{n}", "stage": "PLAY", "do": per_side(off_side, n)})

spec = {
    "format_version": 1,
    "name": "BACKGAMMON",
    "board": {"shape": "map", "places": places, "links": links},
    "players": [{"name": "WHITE", "starting_positions": []}, {"name": "BLACK", "starting_positions": []}],
    "turns": {"order": ["WHITE", "BLACK"], "stages": [{"name": "PLAY"}]},
    "pieces": [{"code": c, "name": c.lower().replace("_", " "), "moves": []}
               for c in ["WHITE_CHECKER", "BLACK_CHECKER"] + [f"D{k}" for k in range(1, 7)]],
    "bags": [
        {"name": "DICE", "for": "EACH_PLAYER", "capacity": 4},
        {"name": "White bar"},
        {"name": "Black bar"},
    ],
    "zones": zones,
    "actions": actions,
    "end_checks": [{"when": {"test": "SOME", "in": {"region": "BOARD"}, "that": {"test": "ALL", "of": [
        {"test": "IN_ZONE", "at": {"place": "EACH"}, "zone": "TRAY"},
        count(at_place("EACH"), None, min=15)]}}, "then": {"outcome": "WIN"}}],
}
out = sys.argv[1] if len(sys.argv) > 1 else "crates/engine/specs/backgammon.json"
open(out, "w").write(json.dumps(spec, indent=2) + "\n")
print("wrote", out)
