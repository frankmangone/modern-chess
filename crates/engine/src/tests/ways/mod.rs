//! The ways of taking an action: every complete list of answers and the changes it makes,
//! worked out by running an action's steps (choices, tests, effects and `if`) on the changing
//! position, and synthesized for a spec still written in the old forms.
mod protocol;

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};
    use std::path::Path;

    use serde_json::json;

    use crate::logic::{Answer, ChoiceKind, Game, Piece, PieceState, Way};
    use crate::shared::{BoardChange, Effect, Position, CAPTURE};
    use crate::specs::{parse_game_spec, parse_game_spec_str};
    use crate::tests::support::{self, promotion_options};

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
            .filter(|way| way.answers.first() == Some(&Answer::Place(from.clone())))
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
                Some(Answer::Place(target)) => Some(target.clone()),
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
        assert_eq!(way.lands, Some(pos![3, 0]));
        assert_eq!(way.from_hand, None);
        assert!(!way.captures && !way.ends_turn);
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
        assert_eq!(way.lands, None, "no piece was picked");
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
        crate::logic::game::board_changes::apply_to_map(&mut game.state.pieces, &way.changes);
        let flag = |game: &Game| game.state.pieces[&pos![3, 3]].state.get("BLESSED").cloned();
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
        assert_eq!(marked.state.get("MARKED"), Some(&PieceState::Blank));
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

        game.state.hand.insert(
            WHITE.to_string(),
            [("PAWN".to_string(), 1), ("ROOK".to_string(), 1)]
                .into_iter()
                .collect(),
        );
        let ways = ways_of(&game, "DROP");
        let hand = |code: &str| Answer::Hand(code.to_string());
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
        assert_eq!(way.kinds, vec![ChoiceKind::Hand, ChoiceKind::Place]);
        assert_eq!(way.from_hand.as_deref(), Some("PAWN"));
        assert_eq!(show(&way.changes), vec!["3,3 PAWN WHITE 0"]);
        assert_eq!(way.lands, Some(pos![3, 3]));
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
        assert_eq!(castle.lands, Some(pos![6, 0]));
    }

    #[test]
    fn set_code_makes_an_unmoved_piece() {
        let mut game = game();
        put_moved(&mut game, pos![2, 2], "PAWN", WHITE, 5);
        if let Some(pawn) = game.state.pieces.get_mut(&pos![2, 2]) {
            pawn.state
                .insert("EN_PASSANT".to_string(), PieceState::Uint(1));
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
        assert!(crowning.ends_turn && crowning.captures);
        assert_eq!(
            show(&crowning.changes),
            vec!["2,6 -", "1,5 -", "3,7 MAN WHITE 1", "3,7 CROWN WHITE 0"]
        );

        let jump = the_way(&ways, &[place(pos![5, 1]), place(pos![7, 3])]);
        assert!(!jump.ends_turn && jump.captures);
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
        assert!(!pass.captures && !pass.ends_turn);
        assert_eq!(pass.lands, None);
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
        let flags: Vec<&String> = twin.expect("a piece").state.keys().collect();
        assert_eq!(flags, vec!["FIRST"]);
    }

    #[test]
    fn a_capture_that_removes_its_own_leader_is_no_threat() {
        // Each king captures one step ahead and blows up with its victim, as in Atomic.
        let spec = json!({
            "name": "BLAST",
            "board": { "dimensions": [4, 4] },
            "leader": ["KING"],
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
            !game.leader_in_check(),
            "taking the white king would blow up the black one"
        );
    }

    #[test]
    fn an_answer_is_written_with_its_kind() {
        let written = serde_json::to_value(vec![
            Answer::Place(pos![4, 2]),
            Answer::Code("QUEEN".to_string()),
            Answer::Hand("PAWN".to_string()),
            Answer::Action("PASS".to_string()),
        ])
        .expect("answers are written");
        assert_eq!(
            written,
            json!([{ "place": "4,2" }, { "code": "QUEEN" }, { "hand": "PAWN" }, { "action": "PASS" }])
        );
        let read: Answer = serde_json::from_value(json!({ "place": "4,2" })).expect("read back");
        assert_eq!(read, Answer::Place(pos![4, 2]));
        assert_eq!(
            serde_json::to_value(ChoiceKind::Move).expect("written"),
            json!("MOVE")
        );
    }

    // -----------------------------------------------------------------------------------
    // The legacy adapter: a spec without `actions` offers what today's turn flow offers.
    // -----------------------------------------------------------------------------------

    /// Where an offer starts: a piece's place, or `drop CODE` for a piece from the hand or a
    /// supply.
    type Offer = (String, String);

    /// What the old turn flow offered the player to move: each piece's destinations as
    /// picking it computed them, and each hand or supply code's squares as picking the code
    /// computed them. Rebuilt here from the pieces that flow used, to compare the ways with.
    fn todays_offers(game: &Game) -> BTreeSet<Offer> {
        let player = game.current_player();
        let mut offers = BTreeSet::new();
        let raw_moves = |origin: &Position| -> HashMap<Position, Effect> {
            let piece = &game.state.pieces[origin];
            game.rules
                .blueprints
                .get(&piece.code)
                .and_then(|blueprint| blueprint.calculate_moves(piece, origin, game))
                .unwrap_or_default()
        };
        let origins: Vec<Position> = game
            .state
            .pieces
            .iter()
            .filter(|(_, piece)| piece.player == player)
            .map(|(position, _)| position.clone())
            .collect();
        let capture_exists = origins
            .iter()
            .any(|origin| raw_moves(origin).values().any(|e| e.action == CAPTURE));
        for origin in &origins {
            // Mid-jump, only the jumping piece could be picked.
            if game
                .state
                .continuing
                .as_ref()
                .is_some_and(|at| at != origin)
            {
                continue;
            }
            let mut moves: HashMap<Position, Effect> = raw_moves(origin)
                .into_iter()
                .filter_map(|(target, effect)| {
                    game.settle_effect(effect).map(|settled| (target, settled))
                })
                .collect();
            game.retain_leader_safe_moves(&mut moves);
            if game.state.continuing.is_some() || (game.rules.mandatory_capture && capture_exists) {
                moves.retain(|_, effect| effect.action == CAPTURE);
            }
            for target in moves.keys() {
                offers.insert((origin.to_string(), target.to_string()));
            }
        }
        let mut codes: Vec<String> = game
            .hand()
            .get(&player)
            .map(|held| {
                held.iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(code, _)| code.clone())
                    .collect()
            })
            .unwrap_or_default();
        codes.extend(game.rules.unlimited_supply.iter().cloned());
        for code in codes {
            for square in game.compute_drop_squares(&code, &player).keys() {
                offers.insert((format!("drop {code}"), square.to_string()));
            }
        }
        offers
    }

    /// The same offers, read from the ways.
    fn offered_by_ways(game: &Game, ways: &[Way]) -> BTreeSet<Offer> {
        ways.iter()
            .filter_map(|way| match way.answers.as_slice() {
                [] => None,
                [Answer::Hand(code), Answer::Place(to)] => {
                    Some((format!("drop {code}"), to.to_string()))
                }
                [Answer::Place(to)] => Some((format!("drop {}", way.action), to.to_string())),
                [Answer::Place(from), Answer::Place(to), ..] => {
                    Some((from.to_string(), to.to_string()))
                }
                other => panic!("{}: unexpected answers {other:?}", game.rules.name),
            })
            .collect()
    }

    fn assert_same_offers(name: &str, when: &str, game: &Game) {
        let ways = game.compute_ways();
        assert_eq!(
            offered_by_ways(game, &ways),
            todays_offers(game),
            "{name} {when}: the ways offer what today's turn flow offers"
        );
        let passes = ways
            .iter()
            .any(|way| way.action == "PASS" && way.answers.is_empty());
        assert_eq!(passes, game.rules.allow_pass, "{name} {when}: a pass");
    }

    /// Plays `offer`, taking a promotion's first option.
    fn play(game: &Game, (origin, target): &Offer) -> Game {
        let mut next = game.clone();
        let target = Position::from(target.as_str());
        match origin.strip_prefix("drop ") {
            Some(code) => support::drop(&mut next, code, target),
            None => support::play(&mut next, Position::from(origin.as_str()), target),
        }
        .expect("an offer can be played");
        if let Some(option) = promotion_options(&next).first() {
            support::promote(&mut next, option).expect("the first option is accepted");
        }
        next
    }

    #[test]
    fn legacy_ways_keep_mandatory_capture_and_the_jump_lock() {
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
        assert_eq!(game.state.continuing, Some(pos![5, 3]));
        assert_eq!(
            pairs(&game),
            vec![(place(pos![5, 3]), place(pos![3, 5]))],
            "only the jumping man's next capture"
        );
    }

    #[test]
    fn legacy_ways_match_todays_moves_for_every_catalogue_spec() {
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
            let game = load(path.to_str().expect("a UTF-8 path"));
            assert!(game.rules.actions.is_empty(), "{name} is in the old forms");
            assert_same_offers(&name, "at the start", &game);

            let first = todays_offers(&game)
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("{name} offers a first move"));
            let after = play(&game, &first);
            assert_same_offers(&name, "after one ply", &after);
        }
    }
}
