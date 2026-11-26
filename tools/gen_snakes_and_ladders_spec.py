"""Writes crates/engine/specs/snakes_and_ladders.json: the squares, the ladders and snakes, and the roll.

Run from the repository root; it rewrites the file it names.
"""
import json
LADDERS = {1:38, 4:14, 9:31, 21:42, 28:84, 36:44, 51:67, 71:91, 80:100}
SNAKES = {16:6, 47:26, 49:11, 56:53, 62:19, 64:60, 87:24, 93:73, 95:75, 98:78}
JUMPS = {**LADDERS, **SNAKES}
def square(n):
    row, col = divmod(n - 1, 10)
    if row % 2 == 1: col = 9 - col
    return col, row
starts = {"RED_TOKEN": 1, "BLUE_TOKEN": 1}
places = [{"name": "0", "at": [-1, 0], "capacity": 2, "start": starts}]
for n in range(1, 101):
    x, y = square(n)
    places.append({"name": str(n), "at": [x, y], "capacity": 2})
links = [{"from": str(n), "to": str(n + 1), "label": "step"} for n in range(0, 100)]
links += [{"from": str(a), "to": str(b), "label": "jump"} for a, b in sorted(JUMPS.items())]
faces = [{"PIP": k} for k in range(1, 7)]
pips = {"count": {"in": {"region": "AT", "at": {"bag": "TRAY"}}, "where": {"codes": ["PIP"]}}}
walk = {"region": "FOLLOW", "from": {"place": "EACH"}, "steps": pips, "through": "step"}
def turn(token):
    mine = {"codes": [token]}
    return {"if": {"test": "PLAYER", "is": [token.split("_")[0]]}, "then": [
        {"effect": "FOR_EACH", "in": {"region": "BOARD", "where": mine}, "do": [
            {"if": {"test": "COUNT", "in": walk, "min": 0}, "then": [
                {"effect": "REMOVE", "at": {"place": "EACH"}, "what": mine, "count": 1},
                {"effect": "FOR_EACH", "in": walk, "do": [
                    {"effect": "REMOVE", "at": {"place": "LAST"}, "what": mine, "count": 1},
                    {"effect": "PLACE", "at": {"place": "EACH"}, "code": token, "count": 1},
                ]},
                {"effect": "FOR_EACH", "in": {"region": "AROUND", "of": {"place": "LAST"}, "through": "jump"}, "do": [
                    {"effect": "REMOVE", "at": {"place": "LAST"}, "what": mine, "count": 1},
                    {"effect": "PLACE", "at": {"place": "EACH"}, "code": token, "count": 1},
                ]},
            ]},
        ]},
    ]}
spec = {
    "format_version": 1,
    "name": "SNAKES AND LADDERS",
    "board": {"shape": "map", "places": places, "links": links},
    "players": [{"name": "RED", "starting_positions": []}, {"name": "BLUE", "starting_positions": []}],
    "turns": {"order": ["RED", "BLUE"]},
    "pieces": [
        {"code": "RED_TOKEN", "name": "red token", "moves": []},
        {"code": "BLUE_TOKEN", "name": "blue token", "moves": []},
        {"code": "PIP", "name": "pip", "moves": []},
    ],
    "bags": [{"name": "TRAY"}],
    "zones": [{"name": "FINISH", "places": {"RED": ["100"], "BLUE": ["100"]}}],
    "actions": [{"name": "ROLL", "do": [
        {"effect": "REMOVE", "at": {"bag": "TRAY"}, "count": "ALL"},
        {"effect": "ROLL", "at": {"bag": "TRAY"}, "faces": faces},
        turn("RED_TOKEN"), turn("BLUE_TOKEN"),
    ]}],
    "end_checks": [{
        "when": {"test": "SOME", "in": {"region": "BOARD"}, "that": {"test": "ALL", "of": [
            {"test": "IN_ZONE", "at": {"place": "EACH"}, "zone": "FINISH"},
            {"test": "COUNT", "in": {"region": "AT", "at": {"place": "EACH"}}, "where": {"owner": "ALLY"}},
        ]}},
        "then": {"outcome": "WIN"},
    }],
}
out = "crates/engine/specs/snakes_and_ladders.json"
open(out, "w").write(json.dumps(spec, indent=2) + "\n")
print("wrote", out)
