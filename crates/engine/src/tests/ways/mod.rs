//! The ways of taking an action: every complete list of answers and the changes it makes,
//! worked out by running an action's steps (choices, tests, effects and `if`) on the changing
//! position.
mod protocol;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use serde_json::json;

    use crate::logic::{Answer, ChoiceKind, Game, GameTransition, Piece, PieceState, Way};
    use crate::shared::{BoardChange, Position};
    use crate::specs::{parse_game_spec, parse_game_spec_str};
    use crate::tests::support;

    const SPEC: &str = "./src/tests/ways/spec.json";
    const TWO_SUPPLIES: &str = "./src/tests/ways/two_supplies.json";
    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";

    fn load(path: &str) -> Game {
        parse_game_spec(path)
            .map(Game::from_spec)
            .unwrap_or_else(|error| panic!("{path} is read: {error:?}"))
    }

    /// The test spec, with nothing on the board and WHITE to move.
    fn game() -> Game {
        load(SPEC)
    }

    fn put(game: &mut Game, at: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(at, Piece::new(code.to_string(), player.to_string()));
    }

    /// A piece that has already moved `moves` times.
    fn put_moved(game: &mut Game, at: Position, code: &str, player: &str, moves: u16) {
        let mut piece = Piece::new(code.to_string(), player.to_string());
        piece.total_moves = moves;
        game.state.pieces.insert(at, piece);
    }

    /// The ways of the action `name`, in the order they were worked out.
    fn ways_of(game: &Game, name: &str) -> Vec<Way> {
        game.compute_ways()
            .into_iter()
            .filter(|way| way.action == name)
            .collect()
    }

    /// The ways whose first answer is the place `from`.
    fn from(ways: &[Way], from: &Position) -> Vec<Way> {
        ways.iter()
            .filter(|way| way.answers.first() == Some(&Answer::Place(*from)))
            .cloned()
            .collect()
    }

    /// The way whose answers are `answers`; there is exactly one.
    fn the_way<'a>(ways: &'a [Way], answers: &[Answer]) -> &'a Way {
        let found: Vec<&Way> = ways.iter().filter(|way| way.answers == answers).collect();
        assert_eq!(found.len(), 1, "one way answers {answers:?}: {found:?}");
        found[0]
    }

    /// The place each way answers second (a `MOVE`'s target).
    fn targets(ways: &[Way]) -> BTreeSet<Position> {
        ways.iter()
            .filter_map(|way| match way.answers.get(1) {
                Some(Answer::Place(target)) => Some(*target),
                _ => None,
            })
            .collect()
    }

    fn place(at: Position) -> Answer {
        Answer::Place(at)
    }

    /// Changes as text: `"4,2 -"` for a removal, `"4,2 PAWN WHITE 1"` (code, owner, moves) for a
    /// piece put on a place.
    fn show(changes: &[BoardChange]) -> Vec<String> {
        changes
            .iter()
            .map(|change| match &change.piece {
                None => format!("{} -", change.position),
                Some(piece) => format!(
                    "{} {} {} {}",
                    change.position, piece.code, piece.player, piece.total_moves
                ),
            })
            .collect()
    }

    fn positions(names: &[Position]) -> BTreeSet<Position> {
        names.iter().cloned().collect()
    }

    #[test]
    fn a_piece_move_gives_one_way_per_reachable_place() {
        let mut game = game();
        put(&mut game, pos![3, 3], "ROOK", WHITE);
        put(&mut game, pos![3, 5], "STONE", WHITE);
        put(&mut game, pos![5, 3], "STONE", BLACK);

        let ways = from(&ways_of(&game, "MOVE"), &pos![3, 3]);
        assert_eq!(
            targets(&ways),
            positions(&[
                pos![3, 4],
                pos![4, 3],
                pos![5, 3],
                pos![3, 2],
                pos![3, 1],
                pos![3, 0],
                pos![2, 3],
                pos![1, 3],
                pos![0, 3],
            ])
        );
        assert_eq!(ways.len(), 9, "one way per place");

        let way = the_way(&ways, &[place(pos![3, 3]), place(pos![3, 0])]);
        assert_eq!(way.kinds, vec![ChoiceKind::Piece, ChoiceKind::Move]);
        assert_eq!(show(&way.changes), vec!["3,3 -", "3,0 ROOK WHITE 1"]);
        assert_eq!(way.last, Some(pos![3, 0]));
        assert_eq!(way.from_hand, None);
        assert!(!way.captures && way.stage.is_none());
    }

    #[test]
    fn a_slide_stops_where_until_says() {
        let mut game = game();
        put(&mut game, pos![2, 0], "SLIDER", WHITE);
        assert_eq!(
            targets(&ways_of(&game, "MOVE")),
            positions(&[pos![2, 1], pos![2, 2], pos![2, 3], pos![2, 4]]),
            "the slide stops on the WALL row"
        );

        // Its `until` names only the wall, so a piece in the way does not stop it; the move's
        // own test leaves out the occupied place.
        put(&mut game, pos![2, 2], "STONE", BLACK);
        assert_eq!(
            targets(&from(&ways_of(&game, "MOVE"), &pos![2, 0])),
            positions(&[pos![2, 1], pos![2, 3], pos![2, 4]])
        );
    }

    #[test]
    fn move_onto_an_enemy_removes_it_and_is_a_capture() {
        let mut game = game();
        put(&mut game, pos![0, 0], "ROOK", WHITE);
        put(&mut game, pos![0, 2], "STONE", BLACK);
        let ways = ways_of(&game, "MOVE");

        let capture = the_way(&ways, &[place(pos![0, 0]), place(pos![0, 2])]);
        assert_eq!(
            show(&capture.changes),
            vec!["0,2 -", "0,0 -", "0,2 ROOK WHITE 1"],
            "the removal of the enemy is in the changes"
        );
        assert!(capture.captures);

        let quiet = the_way(&ways, &[place(pos![0, 0]), place(pos![0, 1])]);
        assert_eq!(show(&quiet.changes), vec!["0,0 -", "0,1 ROOK WHITE 1"]);
        assert!(!quiet.captures);
    }

    #[test]
    fn place_puts_a_new_unmoved_piece_owned_by_the_mover() {
        let mut game = game();
        put(&mut game, pos![0, 0], "STONE", WHITE);
        game.state.current_turn = 1;

        let ways = ways_of(&game, "STONE");
        assert_eq!(ways.len(), 63, "every empty place");
        let way = the_way(&ways, &[place(pos![4, 4])]);
        assert_eq!(way.kinds, vec![ChoiceKind::Place]);
        assert_eq!(show(&way.changes), vec!["4,4 STONE BLACK 0"]);
        assert!(!way.captures);
        assert_eq!(way.last, Some(pos![4, 4]), "the stone was placed there");
    }

    #[test]
    fn set_owner_set_code_and_set_flag_change_the_piece() {
        let mut game = game();
        put_moved(&mut game, pos![1, 1], "ROOK", BLACK, 2);
        put_moved(&mut game, pos![2, 2], "PAWN", WHITE, 1);
        put(&mut game, pos![3, 3], "ROOK", WHITE);

        let converted = ways_of(&game, "CONVERT");
        let way = the_way(&converted, &[place(pos![1, 1])]);
        assert_eq!(show(&way.changes), vec!["1,1 ROOK WHITE 2"]);
        assert!(!way.captures, "nothing left the board");

        let promoted = ways_of(&game, "PROMOTE");
        let way = the_way(&promoted, &[place(pos![2, 2])]);
        assert_eq!(show(&way.changes), vec!["2,2 GOLD WHITE 0"]);

        let blessed = ways_of(&game, "BLESS");
        let way = the_way(&blessed, &[place(pos![3, 3])]).clone();
        assert_eq!(show(&way.changes), vec!["3,3 ROOK WHITE 0"]);

        // `turns: 1` lasts through the next player's turn and is gone after it, as today.
        crate::logic::game::state::board_changes::apply_to_map(
            &mut game.state.pieces,
            &way.changes,
        );
        let flag = |game: &Game| {
            game.state.pieces[&pos![3, 3]]
                .state
                .get(&crate::shared::Symbol::new("BLESSED"))
                .cloned()
        };
        assert_eq!(flag(&game), Some(PieceState::Uint(1)));
        game.next_turn();
        assert!(flag(&game).is_some(), "seen on the opponent's turn");
        game.next_turn();
        assert_eq!(flag(&game), None, "gone after it");
    }

    #[test]
    fn for_each_runs_once_per_place_of_its_region_taken_before_it_runs() {
        let mut game = game();
        put(&mut game, pos![3, 3], "STONE", BLACK);
        put(&mut game, pos![5, 5], "STONE", WHITE);

        let ways = ways_of(&game, "SPREAD");
        let way = the_way(&ways, &[place(pos![4, 4])]);
        let changes: BTreeSet<String> = show(&way.changes).into_iter().collect();
        let expected: BTreeSet<String> = [
            "4,4 STONE WHITE 0",
            "3,3 STONE WHITE 0",
            "3,4 STONE WHITE 0",
            "3,5 STONE WHITE 0",
            "4,3 STONE WHITE 0",
            "4,5 STONE WHITE 0",
            "5,3 STONE WHITE 0",
            "5,4 STONE WHITE 0",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(changes, expected);
        // One change for the target, and one for each place around it but the ally's: the
        // stones put on the way do not add places to go round.
        assert_eq!(way.changes.len(), 8);

        // In a corner there are three places around.
        let corner = the_way(&ways, &[place(pos![0, 0])]);
        assert_eq!(corner.changes.len(), 4);
    }

    #[test]
    fn if_runs_then_or_else() {
        let mut game = game();
        put(&mut game, pos![0, 7], "STONE", WHITE);
        put(&mut game, pos![0, 0], "STONE", WHITE);
        let ways = ways_of(&game, "MARK");

        let then = the_way(&ways, &[place(pos![0, 7])]);
        assert_eq!(show(&then.changes), vec!["0,7 GOLD WHITE 0"]);

        let otherwise = the_way(&ways, &[place(pos![0, 0])]);
        assert_eq!(show(&otherwise.changes), vec!["0,0 STONE WHITE 0"]);
        let marked = otherwise.changes[0].piece.as_ref().expect("a piece");
        assert_eq!(
            marked.state.get(&crate::shared::Symbol::new("MARKED")),
            Some(&PieceState::Blank)
        );
    }

    #[test]
    fn a_test_after_an_effect_sees_the_changed_position() {
        let mut game = game();
        put(&mut game, pos![2, 2], "STONE", BLACK);
        assert_eq!(
            ways_of(&game, "CHECKED").len(),
            63,
            "the stone just put is there for the test after it"
        );
        assert!(
            ways_of(&game, "UNCHECKED").is_empty(),
            "the place is no longer empty once the stone is put"
        );
    }

    #[test]
    fn a_code_choice_after_a_move_gives_one_way_per_option() {
        let mut game = game();
        put(&mut game, pos![1, 6], "PAWN", WHITE);
        put(&mut game, pos![5, 2], "PAWN", WHITE);
        let ways = ways_of(&game, "MOVE");

        let promotions = from(&ways, &pos![1, 6]);
        assert_eq!(promotions.len(), 2);
        let queen = the_way(
            &promotions,
            &[
                place(pos![1, 6]),
                place(pos![1, 7]),
                Answer::Code("QUEEN".to_string()),
            ],
        );
        assert_eq!(
            queen.kinds,
            vec![ChoiceKind::Piece, ChoiceKind::Move, ChoiceKind::Code]
        );
        assert_eq!(
            show(&queen.changes),
            vec!["1,6 -", "1,7 PAWN WHITE 1", "1,7 QUEEN WHITE 0"]
        );
        the_way(
            &promotions,
            &[
                place(pos![1, 6]),
                place(pos![1, 7]),
                Answer::Code("ROOK".to_string()),
            ],
        );

        let plain = from(&ways, &pos![5, 2]);
        assert_eq!(plain.len(), 1, "no choice outside the zone");
        assert_eq!(plain[0].answers.len(), 2);
    }

    #[test]
    fn a_hand_choice_sets_start_and_move_from_it_takes_one_from_the_hand() {
        let mut game = game();
        put(&mut game, pos![0, 0], "STONE", BLACK);
        assert!(ways_of(&game, "DROP").is_empty(), "nothing in the hand");

        game.state.put_in_hand(WHITE, "PAWN", 1);
        game.state.put_in_hand(WHITE, "ROOK", 1);
        let ways = ways_of(&game, "DROP");
        let hand = |code: &str| Answer::Held(code.to_string());
        let of = |code: &str| -> Vec<Way> {
            ways.iter()
                .filter(|way| way.answers.first() == Some(&hand(code)))
                .cloned()
                .collect()
        };
        // START is the piece in the hand: the pawn may not go to the last row, the rook may.
        assert_eq!(of("PAWN").len(), 63 - 8);
        assert_eq!(of("ROOK").len(), 63);

        let way = the_way(&ways, &[hand("PAWN"), place(pos![3, 3])]);
        assert_eq!(way.kinds, vec![ChoiceKind::Held, ChoiceKind::Place]);
        assert_eq!(way.from_hand.as_deref(), Some("PAWN"));
        assert_eq!(show(&way.changes), vec!["3,3 PAWN WHITE 0"]);
        assert_eq!(way.last, Some(pos![3, 3]));
    }

    #[test]
    fn has_moved_counts_only_for_the_picked_piece() {
        let mut game = game();
        put(&mut game, pos![4, 0], "KING", WHITE);
        put(&mut game, pos![7, 0], "ROOK", WHITE);
        let ways = ways_of(&game, "MOVE");

        let castle = the_way(&ways, &[place(pos![4, 0]), place(pos![6, 0])]);
        assert_eq!(
            show(&castle.changes),
            vec!["4,0 -", "6,0 KING WHITE 1", "7,0 -", "5,0 ROOK WHITE 0"],
            "the king counts a move, the rook moved by an effect does not"
        );
        assert_eq!(castle.last, Some(pos![5, 0]), "the rook moved last");
    }

    #[test]
    fn set_code_makes_an_unmoved_piece() {
        let mut game = game();
        put_moved(&mut game, pos![2, 2], "PAWN", WHITE, 5);
        if let Some(pawn) = game.state.pieces.get_mut(&pos![2, 2]) {
            pawn.state.insert(
                crate::shared::Symbol::new("EN_PASSANT"),
                PieceState::Uint(1),
            );
        }
        let ways = ways_of(&game, "PROMOTE");
        let way = the_way(&ways, &[place(pos![2, 2])]);
        let gold = way.changes[0].piece.as_ref().expect("a piece");
        assert_eq!(*gold, Piece::new("GOLD".to_string(), WHITE.to_string()));
    }

    #[test]
    fn end_turn_is_recorded_on_the_way() {
        let mut game = game();
        put(&mut game, pos![1, 5], "MAN", WHITE);
        put(&mut game, pos![2, 6], "STONE", BLACK);
        put(&mut game, pos![5, 1], "MAN", WHITE);
        put(&mut game, pos![6, 2], "STONE", BLACK);
        let ways = ways_of(&game, "MOVE");

        let crowning = the_way(&ways, &[place(pos![1, 5]), place(pos![3, 7])]);
        assert!(crowning.stage.is_none() && crowning.captures);
        assert_eq!(
            show(&crowning.changes),
            vec!["2,6 -", "1,5 -", "3,7 MAN WHITE 1", "3,7 CROWN WHITE 0"]
        );

        let jump = the_way(&ways, &[place(pos![5, 1]), place(pos![7, 3])]);
        assert!(jump.stage.is_some() && jump.captures);
    }

    #[test]
    fn place_then_remove_an_enemy_is_one_turn() {
        let mut game = game();
        put(&mut game, pos![5, 5], "STONE", BLACK);
        let ways = ways_of(&game, "STRIKE");
        assert_eq!(ways.len(), 1);
        assert_eq!(ways[0].answers, vec![place(pos![4, 5])]);
        assert_eq!(show(&ways[0].changes), vec!["4,5 STONE WHITE 0", "5,5 -"]);
        assert!(ways[0].captures);
    }

    #[test]
    fn board_region_lists_every_place_on_a_map() {
        let spec = json!({
            "format_version": 1,
            "name": "MAP WAYS",
            "board": {
                "shape": "map",
                "places": [
                    { "name": "a", "at": [0.0, 0.0] },
                    { "name": "b", "at": [2.0, 0.0] },
                    { "name": "c", "at": [2.0, 2.0] },
                    { "name": "ab", "kind": "path", "ends": ["a", "b"] },
                    { "name": "bc", "kind": "path", "ends": ["b", "c"] },
                    { "name": "f", "kind": "area", "around": ["a", "b", "c"] }
                ]
            },
            "turns": { "order": ["A", "B"] },
            "players": [
                { "name": "A", "starting_positions": [] },
                { "name": "B", "starting_positions": [] }
            ],
            "pieces": [{ "code": "STONE", "name": "stone", "moves": [] }],
            "actions": [{ "name": "PUT", "do": [
                { "choose": "PLACE", "in": { "region": "BOARD" } },
                { "effect": "PLACE", "at": { "place": "TARGET" }, "code": "STONE" }
            ] }]
        });
        let game = parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .expect("the map spec is read");
        let placed: BTreeSet<String> = ways_of(&game, "PUT")
            .iter()
            .map(|way| match way.answers.as_slice() {
                [Answer::Place(at)] => at.to_string(),
                other => panic!("one place: {other:?}"),
            })
            .collect();
        let expected: BTreeSet<String> = ["a", "b", "c", "ab", "bc", "f"]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(placed, expected);
    }

    #[test]
    fn only_options_that_can_finish_are_offered() {
        let mut game = game();
        put(&mut game, pos![3, 3], "DUD", WHITE);
        put(&mut game, pos![0, 0], "ROOK", WHITE);
        let ways = ways_of(&game, "MOVE");
        assert!(
            from(&ways, &pos![3, 3]).is_empty(),
            "the dud's later test always fails, so it is not offered"
        );
        assert!(!from(&ways, &pos![0, 0]).is_empty());
    }

    #[test]
    fn an_empty_action_is_a_pass_with_no_changes() {
        let mut game = game();
        put(&mut game, pos![0, 0], "ROOK", WHITE);
        let ways = ways_of(&game, "PASS");
        assert_eq!(ways.len(), 1);
        let pass = &ways[0];
        assert!(pass.answers.is_empty() && pass.kinds.is_empty() && pass.changes.is_empty());
        assert!(!pass.captures && pass.stage.is_none());
        assert_eq!(pass.last, None);
    }

    #[test]
    fn two_placing_actions_are_told_apart_by_name() {
        let game = load(TWO_SUPPLIES);
        let ways = game.compute_ways();
        assert_eq!(ways.len(), 18, "nine places for each of two actions");
        let on_centre: Vec<&Way> = ways
            .iter()
            .filter(|way| way.answers == vec![place(pos![1, 1])])
            .collect();
        let names: Vec<&str> = on_centre.iter().map(|way| way.action.as_str()).collect();
        assert_eq!(names, vec!["STONE", "DISC"]);
        assert_eq!(show(&on_centre[0].changes), vec!["1,1 STONE WHITE 0"]);
        assert_eq!(show(&on_centre[1].changes), vec!["1,1 DISC WHITE 0"]);
    }

    #[test]
    fn equal_answers_in_one_action_keep_the_first_in_spec_order() {
        let mut game = game();
        put(&mut game, pos![3, 3], "TWIN", WHITE);
        let ways = from(&ways_of(&game, "MOVE"), &pos![3, 3]);
        assert_eq!(
            ways.len(),
            1,
            "two moves reach the same place with the same answers"
        );
        let twin = ways[0]
            .changes
            .last()
            .and_then(|change| change.piece.as_ref());
        let flags: Vec<&crate::shared::Symbol> = twin.expect("a piece").state.keys().collect();
        assert_eq!(flags, vec!["FIRST"]);
    }

    #[test]
    fn a_capture_that_removes_its_own_kept_piece_is_no_threat() {
        // Each king captures one step ahead and blows up with its victim, as in Atomic.
        let spec = json!({
            "format_version": 1,
            "name": "BLAST",
            "board": { "dimensions": [4, 4] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]],
                  "starting_positions": [{ "piece": "KING", "positions": [[0, 0]] }] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]],
                  "starting_positions": [{ "piece": "KING", "positions": [[1, 0]] }] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [{ "code": "KING", "name": "king", "moves": [{ "id": 0, "step": [1, 0], "do": [
                { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ENEMY" } },
                { "effect": "REMOVE", "at": { "place": "TARGET" } },
                { "effect": "REMOVE", "at": { "place": "START" } }
            ] }] }],
            "actions": [{ "name": "MOVE", "do": [
                { "choose": "PIECE", "where": { "owner": "ALLY" } }, { "choose": "MOVE" }
            ] }]
        });
        let game = parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .expect("the spec is read");
        assert!(game.rules.removes_beyond_target);
        assert!(
            !support::in_check(&game, &["KING"]),
            "taking the white king would blow up the black one"
        );
    }

    #[test]
    fn an_answer_is_written_with_its_kind() {
        let written = serde_json::to_value(vec![
            Answer::Place(pos![4, 2]),
            Answer::Code("QUEEN".to_string()),
            Answer::Held("PAWN".to_string()),
            Answer::Action("PASS".to_string()),
        ])
        .expect("answers are written");
        assert_eq!(
            written,
            json!([{ "place": "4,2" }, { "code": "QUEEN" }, { "held": "PAWN" }, { "action": "PASS" }])
        );
        let old: Answer = serde_json::from_value(json!({ "hand": "PAWN" })).expect("old form read");
        assert_eq!(old, Answer::Held("PAWN".to_string()));
        let read: Answer = serde_json::from_value(json!({ "place": "4,2" })).expect("read back");
        assert_eq!(read, Answer::Place(pos![4, 2]));
        assert_eq!(
            serde_json::to_value(ChoiceKind::Move).expect("written"),
            json!("MOVE")
        );
    }

    // -----------------------------------------------------------------------------------
    // The rules that apply to every way, and the catalogue.
    // -----------------------------------------------------------------------------------

    #[test]
    fn the_ways_keep_a_forced_capture_and_the_jump_lock() {
        let mut game = load("./specs/checkers.json");
        game.state.pieces.clear();
        put(&mut game, pos![3, 1], "MAN", "RED");
        put(&mut game, pos![0, 1], "MAN", "RED");
        put(&mut game, pos![4, 2], "MAN", BLACK);
        put(&mut game, pos![4, 4], "MAN", BLACK);
        put(&mut game, pos![0, 7], "MAN", BLACK);
        assert_eq!(game.current_player(), "RED");

        let pairs = |game: &Game| -> Vec<(Answer, Answer)> {
            game.compute_ways()
                .into_iter()
                .map(|way| (way.answers[0].clone(), way.answers[1].clone()))
                .collect()
        };
        assert_eq!(
            pairs(&game),
            vec![(place(pos![3, 1]), place(pos![5, 3]))],
            "a capture hides every other move"
        );

        support::play(&mut game, pos![3, 1], pos![5, 3]).expect("the jump is played");
        assert_eq!(support::jumping(&game), Some(pos![5, 3]));
        assert_eq!(
            pairs(&game),
            vec![(place(pos![5, 3]), place(pos![3, 5]))],
            "only the jumping man's next capture"
        );
    }

    #[test]
    fn every_catalogue_spec_offers_a_first_way_and_plays_it() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("specs");
        let mut names: Vec<String> = std::fs::read_dir(&folder)
            .expect("the specs folder is readable")
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                name.strip_suffix(".json").map(str::to_string)
            })
            .collect();
        names.sort();
        assert!(names.len() > 20, "the catalogue is there");
        for name in names {
            let path = folder.join(format!("{name}.json"));
            let mut game = load(path.to_str().expect("a UTF-8 path"));
            assert!(
                !game.rules.actions.is_empty(),
                "{name} is written in actions"
            );
            let first = game
                .compute_ways()
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("{name} offers a first way"));
            // A way that answers nothing (a pass, a round that starts) is played by naming its action.
            let answers = if first.answers.is_empty() {
                vec![Answer::Action(first.action.clone())]
            } else {
                first.answers
            };
            for answer in answers {
                game.transition(GameTransition::Choose(answer))
                    .unwrap_or_else(|e| panic!("{name}: the first way is playable: {e:?}"));
            }
            assert_eq!(game.history().len(), 1, "{name}: the way was played");
        }
    }
}
