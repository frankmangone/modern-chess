//! What a piece attacks is what its moves could take, judged by the moves' own rules (see
//! `docs/superpowers/specs/2026-11-25-attack-set-legality-design.md`).
//!
//! Two kinds of test: pinned positions, one for each fault the old attack map had (a second,
//! separate geometry that ignored `until` and every condition of a move), and a differential
//! test that compares, piece by piece, the old geometry (kept here, in the tests) with the
//! attack set the engine works out now, over every catalogue game. They must agree wherever a
//! move's only conditions are about its target, and differ only for the pieces the design names.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GamePhase, GameTransition, Piece, PiecesView, Way};
    use crate::shared::{IdSet, Position, Symbol};
    use crate::specs::{parse_game_spec, parse_game_spec_str};

    fn load(name: &str) -> Game {
        parse_game_spec(format!("./specs/{name}.json"))
            .map(Game::from_spec)
            .unwrap_or_else(|error| panic!("{name} loads: {error:?}"))
    }

    fn empty(name: &str) -> Game {
        let mut game = load(name);
        game.state.pieces.clear();
        game
    }

    fn put(game: &mut Game, at: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(at, Piece::new(code.to_string(), player.to_string()));
    }

    fn attacks(game: &Game, player: &str, at: Position) -> bool {
        game.attacked_by(player).contains(&at)
    }

    // -----------------------------------------------------------------------------------------
    // The old geometry: follow the step to the first piece, ignoring `until` and every test
    // but the empty-target heuristic. Kept here as the reference the differential test uses.
    // -----------------------------------------------------------------------------------------

    fn old_threats(game: &Game, player: &str, from: &Position, code: &str) -> IdSet<Position> {
        let mut threats = IdSet::default();
        let Some(blueprint) = game.rules.blueprints.get(&Symbol::new(code)) else {
            return threats;
        };
        let pieces = PiecesView::of(&game.state.pieces);
        for movement in &blueprint.move_blueprints {
            if !(movement.players.is_empty() || movement.players.iter().any(|p| p == player))
                || !movement.threatens
            {
                continue;
            }
            let (Some(step), Some(mut node)) = (movement.step.get(player), game.board.node(from))
            else {
                continue;
            };
            if movement.label_step {
                for &next in game.board.linked(node, step.label) {
                    let Some(target) = game.board.name(next) else {
                        continue;
                    };
                    if !matches!(pieces.get(target), Some(p) if p.player == player) {
                        threats.insert(*target);
                    }
                }
                continue;
            }
            let mut iterations: u8 = 1;
            while let Some(next) = game.board.follow(node, step.label) {
                let Some(target) = game.board.name(next) else {
                    break;
                };
                match pieces.get(target) {
                    Some(p) if p.player == player => break,
                    Some(_) => {
                        threats.insert(*target);
                        break;
                    }
                    None => {
                        threats.insert(*target);
                    }
                }
                if !movement.repeat_options.loop_move && iterations >= movement.repeat_options.times
                {
                    break;
                }
                node = next;
                iterations += 1;
            }
        }
        threats
    }

    fn new_threats(game: &Game, player: &str, from: &Position, code: &str) -> IdSet<Position> {
        let blueprint = &game.rules.blueprints[&Symbol::new(code)];
        blueprint.calculate_threats_with(game, player, from, PiecesView::of(&game.state.pieces))
    }

    // -----------------------------------------------------------------------------------------
    // The faults, one position each. Xiangqi, BLACK attacking a RED piece (BLACK looks down the
    // board).
    // -----------------------------------------------------------------------------------------

    fn xiangqi_with(pieces: &[(Position, &str, &str)]) -> Game {
        let mut game = empty("xiangqi");
        for (at, code, player) in pieces {
            put(&mut game, *at, code, player);
        }
        game
    }

    #[test]
    fn a_cannon_attacks_the_piece_beyond_one_screen_and_no_other() {
        let cannon = (pos![4, 6], "CANNON", "BLACK");
        let target = (pos![4, 0], "GENERAL", "RED");
        let none = xiangqi_with(&[cannon, target]);
        assert!(
            !attacks(&none, "BLACK", pos![4, 0]),
            "with nothing between, there is nothing to jump"
        );
        let one = xiangqi_with(&[cannon, target, (pos![4, 3], "SOLDIER", "RED")]);
        assert!(attacks(&one, "BLACK", pos![4, 0]), "one screen: it takes");
        let ally_screen = xiangqi_with(&[cannon, target, (pos![4, 3], "SOLDIER", "BLACK")]);
        assert!(
            attacks(&ally_screen, "BLACK", pos![4, 0]),
            "a screen of either side"
        );
        let two = xiangqi_with(&[
            cannon,
            target,
            (pos![4, 3], "SOLDIER", "RED"),
            (pos![4, 2], "SOLDIER", "RED"),
        ]);
        assert!(
            !attacks(&two, "BLACK", pos![4, 0]),
            "two pieces between: out of reach"
        );
    }

    #[test]
    fn a_cannon_does_not_attack_the_screen_itself() {
        let game = xiangqi_with(&[
            (pos![4, 6], "CANNON", "BLACK"),
            (pos![4, 3], "SOLDIER", "RED"),
        ]);
        assert!(!attacks(&game, "BLACK", pos![4, 3]));
    }

    #[test]
    fn a_horse_does_not_attack_through_a_blocked_leg() {
        let horse = (pos![5, 2], "HORSE", "BLACK");
        let target = (pos![4, 0], "GENERAL", "RED");
        let open = xiangqi_with(&[horse, target]);
        assert!(attacks(&open, "BLACK", pos![4, 0]), "leg open");
        let blocked = xiangqi_with(&[horse, target, (pos![5, 1], "SOLDIER", "RED")]);
        assert!(!attacks(&blocked, "BLACK", pos![4, 0]), "leg blocked");
    }

    #[test]
    fn an_elephant_does_not_attack_through_its_eye_or_across_the_river() {
        let elephant = (pos![4, 7], "ELEPHANT", "BLACK");
        let prey = (pos![2, 5], "CHARIOT", "RED");
        let open = xiangqi_with(&[elephant, prey]);
        assert!(attacks(&open, "BLACK", pos![2, 5]), "eye open");
        let blocked = xiangqi_with(&[elephant, prey, (pos![3, 6], "SOLDIER", "RED")]);
        assert!(!attacks(&blocked, "BLACK", pos![2, 5]), "eye blocked");
        let across = xiangqi_with(&[
            (pos![4, 5], "ELEPHANT", "BLACK"),
            (pos![2, 3], "CHARIOT", "RED"),
        ]);
        assert!(
            !attacks(&across, "BLACK", pos![2, 3]),
            "an elephant never crosses the river, so it takes nothing there"
        );
    }

    #[test]
    fn a_soldier_attacks_sideways_only_once_across_the_river() {
        let before = xiangqi_with(&[(pos![4, 6], "SOLDIER", "BLACK")]);
        assert!(attacks(&before, "BLACK", pos![4, 5]), "forward, always");
        assert!(
            !attacks(&before, "BLACK", pos![3, 6]),
            "no sideways step yet"
        );
        let across = xiangqi_with(&[(pos![4, 4], "SOLDIER", "BLACK")]);
        assert!(
            attacks(&across, "BLACK", pos![3, 4]),
            "sideways across the river"
        );
        assert!(attacks(&across, "BLACK", pos![5, 4]));
    }

    #[test]
    fn the_flying_general_still_sees_the_other_general_down_an_open_file() {
        let game = xiangqi_with(&[
            (pos![4, 0], "GENERAL", "RED"),
            (pos![4, 9], "GENERAL", "BLACK"),
        ]);
        assert!(attacks(&game, "BLACK", pos![4, 0]));
        assert!(attacks(&game, "RED", pos![4, 9]));
        let between = xiangqi_with(&[
            (pos![4, 0], "GENERAL", "RED"),
            (pos![4, 9], "GENERAL", "BLACK"),
            (pos![4, 4], "SOLDIER", "RED"),
        ]);
        assert!(!attacks(&between, "BLACK", pos![4, 0]));
    }

    fn janggi_with(pieces: &[(Position, &str, &str)]) -> Game {
        let mut game = empty("janggi_cannon");
        for (at, code, player) in pieces {
            put(&mut game, *at, code, player);
        }
        game
    }

    #[test]
    fn a_janggi_cannon_attacks_over_a_non_cannon_screen_only() {
        let cannon = (pos![0, 3], "CANNON", "CHO");
        let prey = (pos![0, 9], "GENERAL", "HAN");
        let chariot = janggi_with(&[cannon, prey, (pos![0, 5], "CHARIOT", "HAN")]);
        assert!(attacks(&chariot, "CHO", pos![0, 9]));
        let over_cannon = janggi_with(&[cannon, prey, (pos![0, 5], "CANNON", "HAN")]);
        assert!(
            !attacks(&over_cannon, "CHO", pos![0, 9]),
            "a cannon is no screen"
        );
        let none = janggi_with(&[cannon, prey]);
        assert!(!attacks(&none, "CHO", pos![0, 9]), "no screen, no jump");
    }

    #[test]
    fn a_janggi_cannon_attacks_no_cannon() {
        let game = janggi_with(&[
            (pos![0, 3], "CANNON", "CHO"),
            (pos![0, 5], "CHARIOT", "HAN"),
            (pos![0, 7], "CANNON", "HAN"),
        ]);
        assert!(!attacks(&game, "CHO", pos![0, 7]));
    }

    // -----------------------------------------------------------------------------------------
    // The general rule, on a small spec of the test's own.
    // -----------------------------------------------------------------------------------------

    fn small(pieces: Value) -> Game {
        let mut spec = json!({
            "format_version": 1,
            "name": "ATTACKS",
            "board": { "dimensions": [5, 5] },
            "zones": [{ "name": "LEFT", "places": {
                "WHITE": [[0, 0], [1, 0]], "BLACK": [[0, 0], [1, 0]]
            } }],
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "actions": [{ "name": "MOVE", "do": [
                { "choose": "PIECE", "where": { "owner": "ALLY" } },
                { "choose": "MOVE" }
            ] }],
            "pieces": []
        });
        spec["pieces"] = pieces;
        let mut game = Game::from_spec(parse_game_spec_str(&spec.to_string()).expect("reads"));
        game.state.pieces.clear();
        game
    }

    fn take() -> Value {
        json!({ "test": "NOT", "of": { "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ALLY" } } })
    }

    fn go_to() -> Value {
        json!({ "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } })
    }

    #[test]
    fn a_slide_stops_where_its_own_until_says_and_not_at_the_first_piece() {
        // Goes on until the square it reaches is empty: past an enemy, to the first free square.
        let mut game = small(json!([{ "code": "HOP", "name": "hop", "moves": [{
            "id": 0, "step": [0, 1],
            "repeat": { "loop": true, "until": { "test": "NOT", "of": { "test": "COUNT",
                "in": { "region": "AT", "at": { "place": "TARGET" } } } } },
            "do": [take(), go_to()]
        }] }, { "code": "MAN", "name": "man", "moves": [] }]));
        put(&mut game, pos![0, 0], "HOP", "WHITE");
        put(&mut game, pos![0, 1], "MAN", "BLACK");
        let set = game.attacked_by("WHITE");
        assert!(set.contains(&pos![0, 1]));
        assert!(
            set.contains(&pos![0, 2]),
            "the slide goes on past the enemy to the empty square its `until` stops at"
        );
        assert!(!set.contains(&pos![0, 3]));
    }

    #[test]
    fn a_move_kept_in_a_zone_attacks_only_inside_it() {
        let mut game = small(json!([{ "code": "BOUND", "name": "bound", "moves": [{
            "id": 0, "step": [1, 0], "repeat": { "loop": true },
            "do": [take(),
                { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "LEFT" }, go_to()]
        }] }]));
        put(&mut game, pos![0, 0], "BOUND", "WHITE");
        let set = game.attacked_by("WHITE");
        assert!(set.contains(&pos![1, 0]));
        assert!(!set.contains(&pos![2, 0]), "outside the zone: not attacked");
        assert!(!set.contains(&pos![3, 0]));
    }

    #[test]
    fn a_push_onto_an_empty_square_attacks_nothing_and_a_capture_does() {
        let mut game = small(json!([{ "code": "PAWN", "name": "pawn", "moves": [
            { "id": 0, "step": [0, 1], "do": [
                { "test": "NOT", "of": { "test": "COUNT",
                    "in": { "region": "AT", "at": { "place": "TARGET" } } } }, go_to()] },
            { "id": 1, "step": [1, 1], "do": [
                { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } },
                  "where": { "owner": "ENEMY" } }, go_to()] }
        ] }]));
        put(&mut game, pos![2, 2], "PAWN", "WHITE");
        let set = game.attacked_by("WHITE");
        assert!(!set.contains(&pos![2, 3]), "a push takes nothing");
        assert!(
            set.contains(&pos![3, 3]),
            "an empty square its capture would reach is attacked"
        );
    }

    #[test]
    fn a_test_that_wants_a_code_on_the_target_fails_for_an_empty_square() {
        // Takes only kings: an empty square holds no king, so it is not attacked; a king is.
        let mut game = small(json!([
            { "code": "KINGSLAYER", "name": "slayer", "moves": [{ "id": 0, "step": [1, 0],
                "do": [{ "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } },
                         "where": { "owner": "ENEMY", "codes": ["KING"] } }, go_to()] }] },
            { "code": "KING", "name": "king", "moves": [] }
        ]));
        put(&mut game, pos![0, 0], "KINGSLAYER", "WHITE");
        assert!(!game.attacked_by("WHITE").contains(&pos![1, 0]));
        put(&mut game, pos![1, 0], "KING", "BLACK");
        assert!(game.attacked_by("WHITE").contains(&pos![1, 0]));
    }

    #[test]
    fn a_condition_that_reads_the_attack_map_is_not_applied_to_threats() {
        // A step that may not be made from an attacked square (castling out of check) still
        // attacks the squares it reaches: `ATTACKED` in a move's conditions holds, so it does
        // not recurse and is not applied.
        let mut game = small(json!([
            { "code": "SHY", "name": "shy", "moves": [{
                "id": 0, "step": [1, 0],
                "do": [take(), { "test": "NOT", "of": { "test": "ATTACKED",
                    "in": { "region": "AT", "at": { "place": "START" } } } }, go_to()]
            }] },
            { "code": "RAY", "name": "ray", "moves": [{
                "id": 0, "step": [0, 1], "repeat": { "loop": true }, "do": [take(), go_to()]
            }] }
        ]));
        put(&mut game, pos![0, 0], "SHY", "WHITE");
        // BLACK's ray looks down the file that holds SHY, which is therefore attacked.
        put(&mut game, pos![0, 4], "RAY", "BLACK");
        assert!(game.attacked_by("BLACK").contains(&pos![0, 0]));
        assert!(game.attacked_by("WHITE").contains(&pos![1, 0]));
    }

    // -----------------------------------------------------------------------------------------
    // The differential test: the old geometry against the engine's attack sets, piece by piece.
    // -----------------------------------------------------------------------------------------

    /// The pieces, by catalogue game, whose attack set may differ from the old geometry: those
    /// whose moves carry a condition that is not about the target.
    const MAY_DIFFER: &[(&str, &[&str])] = &[
        // The horse (leg), elephant (eye, river), cannon (screen), soldier (sideways only across
        // the river), advisor and general (the palace). Not the chariot: it has no condition.
        (
            "xiangqi",
            &[
                "HORSE", "ELEPHANT", "CANNON", "SOLDIER", "ADVISOR", "GENERAL",
            ],
        ),
        ("mini_xiangqi", &["CANNON", "SOLDIER", "GENERAL"]),
        ("janggi_cannon", &["CANNON"]),
        // Every move of Solidarity needs an ally beside the piece, so a piece alone attacks
        // nothing.
        ("solidarity", &["*"]),
    ];

    fn may_differ(game: &str, code: &str) -> bool {
        MAY_DIFFER
            .iter()
            .any(|(name, codes)| *name == game && (codes.contains(&"*") || codes.contains(&code)))
    }

    /// The first way of each distinct (first answer, second answer) of the ways of `game`.
    fn firsts(game: &mut Game) -> Vec<Way> {
        let ways = game.state.ways.take().unwrap_or_default();
        let mut seen = std::collections::HashSet::new();
        ways.into_iter()
            .filter(|way| {
                let key = (
                    way.action.clone(),
                    way.answers.first().cloned(),
                    way.answers.get(1).cloned(),
                );
                !way.answers.is_empty() && seen.insert(key)
            })
            .collect()
    }

    /// Plays `way` on a copy of `game`, giving the answers the turn has not been given yet (a
    /// piece that must go on jumping is already picked).
    fn play(game: &Game, way: &Way) -> Game {
        let given = crate::tests::support::chosen(game).len();
        let mut next = game.clone();
        next.state.ways = Some(vec![way.clone()]);
        for answer in way.answers.iter().skip(given) {
            next.transition(GameTransition::Choose(answer.clone()))
                .unwrap_or_else(|error| panic!("{answer:?} was refused: {error:?}"));
        }
        next
    }

    /// Compares every piece's old and new attack set in `game`; the differences found, by code.
    fn compare(name: &str, game: &Game, differing: &mut Vec<(String, String)>) {
        for (at, piece) in &game.state.pieces {
            let old = old_threats(game, &piece.player, at, &piece.code);
            let new = new_threats(game, &piece.player, at, &piece.code);
            if old != new {
                assert!(
                    may_differ(name, &piece.code),
                    "{name}: a {} of {} at {at} attacks differently now\n old {old:?}\n new {new:?}",
                    piece.code,
                    piece.player,
                );
                differing.push((name.to_string(), piece.code.to_string()));
            }
        }
    }

    fn walk(name: &str, game: &mut Game, plies: usize, differing: &mut Vec<(String, String)>) {
        compare(name, game, differing);
        if plies == 0 || matches!(game.state.phase, GamePhase::GameOver { .. }) {
            return;
        }
        for way in firsts(game) {
            walk(name, &mut play(game, &way), plies - 1, differing);
        }
    }

    /// A few long lines of play, picking the way by a small xorshift generator.
    fn wander(
        name: &str,
        game: &Game,
        seed: u64,
        length: usize,
        differing: &mut Vec<(String, String)>,
    ) {
        let mut game = game.clone();
        let mut state = seed | 1;
        for _ in 0..length {
            if matches!(game.state.phase, GamePhase::GameOver { .. }) {
                return;
            }
            let ways = firsts(&mut game);
            if ways.is_empty() {
                return;
            }
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let way = &ways[(state % ways.len() as u64) as usize];
            game = play(&game, way);
            compare(name, &game, differing);
        }
    }

    #[test]
    fn the_attack_sets_agree_with_the_old_geometry_except_for_the_named_pieces() {
        let mut checked = 0;
        let mut differing: Vec<(String, String)> = Vec::new();
        let mut names: Vec<String> = std::fs::read_dir("./specs")
            .expect("the catalogue")
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                name.strip_suffix(".json").map(str::to_string)
            })
            .collect();
        names.sort();
        for name in names {
            let mut game = load(&name);
            // A game whose pieces have no moves (Go, Hex, Kalah, rock-paper-scissors) has no
            // attack set to compare.
            if game
                .rules
                .blueprints
                .values()
                .all(|bp| bp.move_blueprints.is_empty())
            {
                continue;
            }
            // Every first move, then a few long lines of play. The games with a condition that
            // is not about the target get more lines, since that is where the sets may differ.
            let (lines, length) = if MAY_DIFFER.iter().any(|(game, _)| *game == name) {
                (8, 30)
            } else {
                (2, 16)
            };
            walk(&name, &mut game, 1, &mut differing);
            for seed in 1..=lines {
                wander(&name, &load(&name), seed, length, &mut differing);
            }
            checked += 1;
        }
        assert!(checked >= 20);
        // The test must not pass for want of looking: the pieces named do differ somewhere.
        for (name, code) in [("xiangqi", "CANNON"), ("xiangqi", "HORSE")] {
            assert!(
                differing.iter().any(|(n, c)| n == name && c == code),
                "no position in {name} showed a different {code}: the test looked at too little"
            );
        }
    }
}
