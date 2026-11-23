"""Rewrites game specs from `win_conditions`, `draw_conditions`, `leader` and `stalemate_loses`
to `end_checks`, `counters`, `score` and `every_action` entries, by text surgery so that the
formatting of the rest of the file is kept. Usage: convert_ends.py [--keeping NAME,...] FILE..."""
import itertools
import json
import sys

OLD_KEYS = ["leader", "stalemate_loses", "win_conditions", "draw_conditions"]


# ---------------------------------------------------------------- text surgery

def scan_top_level(text):
    """Maps each top-level key to (start of the key, end of its value) in `text`."""
    spans, depth, i, n = {}, 0, 0, len(text)
    while i < n:
        ch = text[i]
        if ch == '"':
            j = i + 1
            while text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            if depth == 1:
                k = j + 1
                while text[k] in " \t\r\n":
                    k += 1
                if text[k] == ":":
                    key = text[i + 1 : j]
                    v = k + 1
                    while text[v] in " \t\r\n":
                        v += 1
                    spans[key] = (i, value_end(text, v))
                    i = spans[key][1]
                    continue
            i = j + 1
            continue
        if ch in "{[":
            depth += 1
        elif ch in "}]":
            depth -= 1
        i += 1
    return spans


def value_end(text, v):
    if text[v] in "{[":
        depth, i = 0, v
        while True:
            ch = text[i]
            if ch == '"':
                i += 1
                while text[i] != '"':
                    i += 2 if text[i] == "\\" else 1
            elif ch in "{[":
                depth += 1
            elif ch in "}]":
                depth -= 1
                if depth == 0:
                    return i + 1
            i += 1
    if text[v] == '"':
        i = v + 1
        while text[i] != '"':
            i += 2 if text[i] == "\\" else 1
        return i + 1
    i = v
    while text[i] not in ",}\n \t\r":
        i += 1
    return i


def delete_key(text, key):
    spans = scan_top_level(text)
    if key not in spans:
        return text
    start, end = spans[key]
    # Take the line's indentation before it, and the comma after (or before) it.
    line_start = start
    while line_start > 0 and text[line_start - 1] in " \t":
        line_start -= 1
    j = end
    while text[j] in " \t":
        j += 1
    if text[j] == ",":
        j += 1
        while text[j] in " \t":
            j += 1
        if text[j] == "\n":
            j += 1
        return text[:line_start] + text[j:]
    # Last key: remove the comma before it instead.
    k = line_start
    while text[k - 1] in " \t\r\n":
        k -= 1
    if text[k - 1] == ",":
        k -= 1
    return text[:k] + text[end:]


def oneline(obj):
    if isinstance(obj, dict):
        if not obj:
            return "{}"
        return "{ " + ", ".join(json.dumps(k) + ": " + oneline(v) for k, v in obj.items()) + " }"
    if isinstance(obj, list):
        return "[" + ", ".join(oneline(v) for v in obj) + "]"
    return json.dumps(obj)


def fmt(obj, indent, prefix=0, width=88):
    """Prints `obj` on one line when it fits after `prefix` characters on a line indented by
    `indent`, and otherwise one entry per line."""
    one = oneline(obj)
    if indent + prefix + len(one) <= width or not isinstance(obj, (dict, list)) or not obj:
        return one
    pad = " " * (indent + 2)
    if isinstance(obj, dict):
        rows = []
        for k, v in obj.items():
            key = json.dumps(k) + ": "
            rows.append(pad + key + fmt(v, indent + 2, len(key)))
        return "{\n" + ",\n".join(rows) + "\n" + " " * indent + "}"
    rows = [pad + fmt(v, indent + 2) for v in obj]
    return "[\n" + ",\n".join(rows) + "\n" + " " * indent + "]"


def indent_block(obj, indent="  ", prefix=0):
    return fmt(obj, len(indent), prefix)


def add_key(text, key, value):
    spans = scan_top_level(text)
    end = text.rstrip().rfind("}")
    before = text[:end].rstrip()
    sep = "" if before.endswith("{") else ","
    return before + sep + f'\n  "{key}": ' + fmt(value, 2, len(key) + 4) + "\n" + text[end:]


def extend_array(text, key, items):
    spans = scan_top_level(text)
    start, end = spans[key]
    close = end - 1
    assert text[close] == "]", key
    before = text[:close].rstrip()
    body = ",\n".join("    " + fmt(item, 4) for item in items)
    sep = "" if before.endswith("[") else ","
    return before + sep + "\n" + body + "\n  " + text[close:]


# ---------------------------------------------------------------- blocks

def at(place):
    return {"place": place}


def count(owner=None, codes=None, except_=None, region=None, min_=None, max_=None, **flags):
    where = {}
    if owner:
        where["owner"] = owner
    if codes:
        where["codes"] = codes
    if except_:
        where["except"] = except_
    where.update(flags)
    test = {"test": "COUNT", "in": region or {"region": "BOARD"}}
    if where:
        test["where"] = where
    if min_ is not None:
        test["min"] = min_
    if max_ is not None:
        test["max"] = max_
    return test


def none_of(owner=None, codes=None, except_=None):
    return count(owner, codes, except_, min_=0, max_=0)


def some(region, that):
    return {"test": "SOME", "in": region, "that": that}


def board_where(owner, codes=None):
    where = {"owner": owner}
    if codes:
        where["codes"] = codes
    return {"region": "BOARD", "where": where}


def check(when, outcome, as_=None):
    entry = {}
    if as_:
        entry["as"] = as_
    entry["when"] = when
    entry["then"] = {"outcome": outcome}
    return entry


def piece_in_zone(rule):
    that = {"test": "IN_ZONE", "at": at("EACH"), "zone": rule["zone"]}
    return check(some(board_where("ALLY", [rule["piece"]]), that), "WIN")


def opponent_bare(rule):
    return check(none_of("ENEMY", except_=rule.get("exempt") or None), "WIN")


def connection(rule):
    pieces = rule.get("pieces") or []
    zones = rule["zones"]
    through = rule["through"]
    group_where = {"codes": pieces} if pieces else None

    def touches(zone):
        group = {"region": "GROUP", "of": at("EACH"), "through": through}
        if group_where:
            group["where"] = group_where
        return some(group, {"test": "IN_ZONE", "at": at("EACH"), "zone": zone})

    needed = rule.get("at_least") or len(zones)
    combos = [
        {"test": "ALL", "of": [touches(z) for z in combo]}
        for combo in itertools.combinations(zones, needed)
    ]
    chain = combos[0] if len(combos) == 1 else {"test": "ANY", "of": combos}
    return check(some(board_where("ALLY", pieces), chain), "WIN")


def insufficient(entries):
    def side(owner):
        options = []
        for entry in entries:
            tests = []
            for code in sorted(set(entry)):
                n = entry.count(code)
                tests.append(count(owner, [code], min_=n, max_=n))
            tests.append(none_of(owner, except_=sorted(set(entry))))
            options.append({"test": "ALL", "of": tests})
        return options[0] if len(options) == 1 else {"test": "ANY", "of": options}

    return check({"test": "ALL", "of": [side("ALLY"), side("ENEMY")]}, "DRAW")


def convert_spec(d, keeping):
    """The new keys for spec `d`: end_checks, counters, score, and the every_action entries to
    append."""
    leader = d.get("leader") or []
    players = [p["name"] for p in d["players"]]
    checks, counters, score, entries = [], [], [], []
    wins = d.get("win_conditions") or []

    if keeping and leader:
        checks.append(
            check(none_of("ALLY", leader), "LOSE", as_="NEXT")
        )
    for rule in wins:
        kind = rule["type"]
        if kind == "PIECE_IN_ZONE":
            checks.append(piece_in_zone(rule))
        elif kind == "OPPONENT_BARE":
            checks.append(opponent_bare(rule))
        elif kind == "CONNECTION":
            checks.append(connection(rule))
        elif kind == "CHECK_COUNT":
            counters.append("CHECKS")
            entries.append(
                {"do": [{
                    "if": {"test": "ATTACKED", "in": board_where("ENEMY", leader), "by": "MOVER"},
                    "then": [{"effect": "ADD", "counter": "CHECKS"}],
                }]}
            )
            checks.append(
                check({"test": "COUNTER", "name": "CHECKS", "min": rule.get("threshold") or 3}, "WIN")
            )
        elif kind == "AREA_SCORE":
            pieces = rule.get("pieces") or []
            where = {"where": {"codes": pieces}} if pieces else {}
            score.append({"term": "PIECES", **where})
            if rule.get("territory"):
                score.append({"term": "TERRITORY", **where})
            for player, value in (rule.get("bonus") or {}).items():
                score.append({"term": "POINTS", "value": value, "to": [player]})
            passes = rule.get("after_passes") or len(players)
            checks.append(
                check({"test": "PASSES", "in_a_row": max(passes, 1)}, "HIGHEST")
            )
        else:
            raise SystemExit(f"unknown win condition {kind}")

    draws = d.get("draw_conditions") or {}
    if draws.get("repetition_count"):
        checks.append(check({"test": "REPEATS", "times": draws["repetition_count"]}, "DRAW"))
    if draws.get("fifty_move_halfmoves"):
        counters.append("QUIET")
        resets = [{"test": "REMOVED", "where": {"owner": "ENEMY"}}]
        if draws.get("fifty_move_pawn_codes"):
            resets.append({"test": "MOVED", "where": {"codes": draws["fifty_move_pawn_codes"]}})
        entries.append(
            {"do": [{
                "if": {"test": "ANY", "of": resets},
                "then": [{"effect": "RESET", "counter": "QUIET", "to": "ALL"}],
                "else": [{"effect": "ADD", "counter": "QUIET", "to": "ALL"}],
            }]}
        )
        checks.append(
            check({"test": "COUNTER", "name": "QUIET", "min": draws["fifty_move_halfmoves"]}, "DRAW")
        )
    if draws.get("insufficient_material"):
        checks.append(insufficient(draws["insufficient_material"]))

    if d.get("stalemate_loses"):
        checks.append(check({"test": "STUCK"}, "LOSE", as_="NEXT"))
    elif leader:
        attacked = {"test": "ATTACKED", "in": board_where("ALLY", leader), "by": "ENEMY"}
        if keeping:
            attacked["keeping"] = {"codes": leader}
        checks.append(
            check({"test": "ALL", "of": [{"test": "STUCK"}, attacked]}, "LOSE", as_="NEXT")
        )
        checks.append(check({"test": "STUCK"}, "DRAW", as_="NEXT"))
    return checks, counters, score, entries


def add_keeping(node, leader):
    """Adds `keeping` to every ATTACKED test that is by the enemy (the default)."""
    if isinstance(node, dict):
        if node.get("test") == "ATTACKED" and node.get("by") != "MOVER" and "keeping" not in node:
            node["keeping"] = {"codes": leader}
        for v in node.values():
            add_keeping(v, leader)
    elif isinstance(node, list):
        for v in node:
            add_keeping(v, leader)


def main():
    args = sys.argv[1:]
    keeping_names = set()
    if args and args[0] == "--keeping":
        keeping_names = set(args[1].split(","))
        args = args[2:]
    for path in args:
        text = open(path).read()
        d = json.loads(text)
        if not any(k in d for k in OLD_KEYS):
            print("skip   ", path)
            continue
        base = path.rsplit("/", 1)[-1]
        keeping = base in keeping_names
        checks, counters, score, entries = convert_spec(d, keeping)
        leader = d.get("leader") or []
        for key in OLD_KEYS:
            text = delete_key(text, key)
        if counters:
            text = add_key(text, "counters", counters)
        if score:
            text = add_key(text, "score", score)
        if checks:
            text = add_key(text, "end_checks", checks)
        if entries:
            if "every_action" in d:
                text = extend_array(text, "every_action", entries)
            else:
                text = add_key(text, "every_action", entries)
        new = json.loads(text)
        for key in OLD_KEYS:
            assert key not in new, (path, key)
        assert new["actions"] == d["actions"], path
        if keeping and leader:
            node = new.get("every_action", [])
            add_keeping(node, leader)
            # rewrite the every_action array with keeping added
            text = delete_key(text, "every_action")
            text = add_key(text, "every_action", node)
            new = json.loads(text)
        open(path, "w").write(text)
        print("convert", path, "checks:", len(checks), "counters:", counters, "entries:", len(entries))


main()
