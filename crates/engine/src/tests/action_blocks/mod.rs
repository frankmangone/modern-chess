//! Actions and steps as blocks: each choice, effect and `if` read from a spec and compiled for
//! each player, the `BOARD` region and the `EACH` place, every refusal when the spec is read,
//! and what is noted about a move when it is compiled (`threatens`, `removes_beyond_target`).
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::{json, Value};

    use crate::logic::places::place::Anchor;
    use crate::logic::rules::steps::{Choice, CodeRef, EffectStep, Step};
    use crate::logic::rules::test_block::{judge, Region, Test, TestContext};
    use crate::logic::{Game, Labels, Piece};
    use crate::shared::Position;
    use crate::specs::game::test_block::{RegionSpec, TestSpec};
    use crate::specs::game::CodeSpec;
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpec, GameSpecError};

    const SPEC: &str = "./src/tests/action_blocks/spec.json";
    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";

    fn load_spec() -> GameSpec {
        parse_game_spec(SPEC).expect("the action_blocks spec is read")
    }

    fn load_game() -> Game {
        Game::from_spec(load_spec())
    }

    fn spec_value() -> Value {
        let text = std::fs::read_to_string(SPEC).expect("the spec file is there");
        serde_json::from_str(&text).expect("the spec is JSON")
    }

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    /// No ally stands on `TARGET`.
    fn not_ally() -> Value {
        json!({ "test": "NOT", "of": {
            "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "TARGET" } },
            "where": { "owner": "ALLY" }
        } })
    }

    /// The piece at `START` goes to `TARGET`.
    fn move_effect() -> Value {
        json!({ "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } })
    }

    /// A 4×4 game whose PROBE steps [0, 1] onto a square no ally holds, with `actions`.
    fn small_spec(actions: Value) -> Value {
        json!({
            "name": "STEPS",
            "board": { "dimensions": [4, 4] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "bags": [{ "name": "HAND", "for": "EACH_PLAYER" }],
            "pieces": [{
                "code": "PROBE",
                "name": "probe",
                "moves": [{ "id": 0, "step": [0, 1], "do": [not_ally(), move_effect()] }]
            }],
            "actions": actions
        })
    }

    /// `small_spec` with the one action PASS, an action with no steps.
    fn with_a_pass() -> Value {
        small_spec(json!([{ "name": "PASS", "do": [] }]))
    }

    /// `small_spec` with the one action `A`, whose steps are `steps`.
    fn one_action(steps: Value) -> Value {
        small_spec(json!([{ "name": "A", "do": steps }]))
    }

    /// The kinds of the steps written in `steps`, nested ones included, read from the JSON.
    fn written_kinds(steps: &Value, kinds: &mut BTreeSet<String>) {
        for step in steps.as_array().into_iter().flatten() {
            if let Some(kind) = step.get("choose").and_then(Value::as_str) {
                kinds.insert(format!("choose {kind}"));
            } else if let Some(kind) = step.get("effect").and_then(Value::as_str) {
                kinds.insert(format!("effect {kind}"));
                written_kinds(&step["do"], kinds);
            } else if step.get("if").is_some() {
                let has_else = step.get("else").is_some();
                kinds.insert(if has_else { "if, else" } else { "if" }.to_string());
                written_kinds(&step["then"], kinds);
                written_kinds(&step["else"], kinds);
            } else if step.get("test").is_some() {
                kinds.insert("test".to_string());
            }
        }
    }

    /// The kinds of the compiled `steps`, nested ones included.
    fn compiled_kinds(steps: &[Step], kinds: &mut BTreeSet<String>) {
        for step in steps {
            let kind = match step {
                Step::Choose(choice) => match choice {
                    Choice::Piece(_) => "choose PIECE",
                    Choice::Move => "choose MOVE",
                    Choice::Place(_) => "choose PLACE",
                    Choice::Held { .. } => "choose HELD",
                    Choice::Code { .. } => "choose CODE",
                },
                Step::Test(_) => "test",
                Step::Effect(effect) => match effect {
                    EffectStep::Move { .. } => "effect MOVE",
                    EffectStep::Remove { .. } => "effect REMOVE",
                    EffectStep::Put { .. } => "effect PLACE",
                    EffectStep::SetOwner(_) => "effect SET_OWNER",
                    EffectStep::SetCode { .. } => "effect SET_CODE",
                    EffectStep::SetFlag { .. } => "effect SET_FLAG",
                    EffectStep::ForEach { steps, .. } => {
                        compiled_kinds(steps, kinds);
                        "effect FOR_EACH"
                    }
                    EffectStep::Add { .. } => "effect ADD",
                    EffectStep::Reset { .. } => "effect RESET",
                    EffectStep::Stage(_) => "effect STAGE",
                },
                Step::If {
                    then, otherwise, ..
                } => {
                    compiled_kinds(then, kinds);
                    compiled_kinds(otherwise, kinds);
                    if otherwise.is_empty() {
                        "if"
                    } else {
                        "if, else"
                    }
                }
            };
            kinds.insert(kind.to_string());
        }
    }

    /// The compiled steps of move `id` of piece `code`, for `player`.
    fn move_steps<'a>(game: &'a Game, code: &str, id: u8, player: &str) -> &'a [Step] {
        game.rules.blueprints[&crate::shared::Symbol::new(code)]
            .move_blueprints
            .iter()
            .find(|movement| movement.id == id)
            .and_then(|movement| movement.steps.get(player))
            .expect("the move is compiled for the player")
    }

    #[test]
    fn every_step_kind_parses() {
        let value = spec_value();
        let mut written = BTreeSet::new();
        for piece in value["pieces"].as_array().into_iter().flatten() {
            for movement in piece["moves"].as_array().into_iter().flatten() {
                written_kinds(&movement["do"], &mut written);
            }
        }
        for action in value["actions"].as_array().into_iter().flatten() {
            written_kinds(&action["do"], &mut written);
        }
        let expected: BTreeSet<String> = [
            "choose PIECE",
            "choose MOVE",
            "choose PLACE",
            "choose HELD",
            "choose CODE",
            "test",
            "effect MOVE",
            "effect REMOVE",
            "effect PLACE",
            "effect SET_OWNER",
            "effect SET_CODE",
            "effect SET_FLAG",
            "effect FOR_EACH",
            "effect STAGE",
            "if",
            "if, else",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(written, expected, "the spec writes every kind of step");

        let game = load_game();
        let mut compiled = BTreeSet::new();
        for blueprint in game.rules.blueprints.values() {
            for movement in &blueprint.move_blueprints {
                compiled_kinds(&movement.steps[WHITE], &mut compiled);
            }
        }
        for action in &game.rules.actions {
            compiled_kinds(&action.steps[WHITE], &mut compiled);
        }
        assert_eq!(compiled, expected, "every kind of step is compiled");
    }

    #[test]
    fn actions_are_compiled_for_each_player_who_may_take_them() {
        let game = load_game();
        let names: Vec<&str> = game
            .rules
            .actions
            .iter()
            .map(|action| action.name.as_str())
            .collect();
        assert_eq!(names, ["MOVE", "DROP", "STONE", "BLESS", "PASS"]);

        let players_of = |name: &str| -> BTreeSet<String> {
            let action = game.rules.actions.iter().find(|a| a.name == name);
            action
                .expect("the action is compiled")
                .steps
                .keys()
                .cloned()
                .collect()
        };
        assert_eq!(
            players_of("MOVE"),
            BTreeSet::from([WHITE.into(), BLACK.into()])
        );
        assert_eq!(players_of("BLESS"), BTreeSet::from([WHITE.into()]));
        let pass = game.rules.actions.iter().find(|a| a.name == "PASS");
        assert!(
            pass.is_some_and(|pass| pass.steps.values().all(Vec::is_empty)),
            "a pass has no steps"
        );

        // En passant removes the piece beside START, at [1, 0] turned for each player: each
        // player's steps hold only that player's offset.
        let removed = |player: &str| match &move_steps(&game, "PAWN", 3, player)[2] {
            Step::Effect(EffectStep::Remove { at, .. }) => {
                assert_eq!(at.anchor, Anchor::Start);
                let offsets = at.offset.as_ref().expect("an offset is written");
                assert_eq!(offsets.keys().collect::<Vec<_>>(), [player]);
                offsets[player]
            }
            other => panic!("the third step of en passant is a REMOVE: {other:?}"),
        };
        assert_ne!(
            removed(WHITE),
            removed(BLACK),
            "the offset is turned for BLACK"
        );
    }

    #[test]
    fn a_code_is_a_string_or_a_chosen_name() {
        let code: CodeSpec = serde_json::from_value(json!("CROWN")).expect("a string");
        assert_eq!(code, CodeSpec::Code("CROWN".into()));
        let chosen: CodeSpec =
            serde_json::from_value(json!({ "chosen": "promotion" })).expect("a chosen name");
        assert_eq!(
            chosen,
            CodeSpec::Chosen {
                chosen: "promotion".into()
            }
        );
        for wrong in [
            json!({ "chosen": "promotion", "also": 1 }),
            json!(3),
            json!({}),
        ] {
            assert!(
                serde_json::from_value::<CodeSpec>(wrong.clone()).is_err(),
                "{wrong} is no code"
            );
        }

        let game = load_game();
        let set_code = |steps: &[Step]| -> Option<CodeRef> {
            steps.iter().find_map(|step| match step {
                Step::If { then, .. } => then.iter().find_map(|step| match step {
                    Step::Effect(EffectStep::SetCode { code, .. }) => Some(code.clone()),
                    _ => None,
                }),
                _ => None,
            })
        };
        assert_eq!(
            set_code(move_steps(&game, "PAWN", 0, WHITE)),
            Some(CodeRef::Chosen("promotion".into()))
        );
        assert_eq!(
            set_code(move_steps(&game, "MAN", 0, WHITE)),
            Some(CodeRef::Code("CROWN".into()))
        );
    }

    #[test]
    fn board_region_and_each_place_parse() {
        let board: RegionSpec =
            serde_json::from_value(json!({ "region": "BOARD" })).expect("BOARD is a region");
        assert!(matches!(board, RegionSpec::Board { r#where: None }));
        let error = serde_json::from_value::<RegionSpec>(
            json!({ "region": "BOARD", "of": { "place": "TARGET" } }),
        )
        .expect_err("BOARD takes no fields");
        assert!(error.to_string().contains("unknown field `of`"), "{error}");

        let game = load_game();
        let drop = game.rules.actions.iter().find(|a| a.name == "DROP");
        let steps = &drop.expect("DROP is compiled").steps[WHITE];
        assert!(
            matches!(
                &steps[1],
                Step::Choose(Choice::Place(Region::Board { filter: None }))
            ),
            "DROP chooses a place of the BOARD: {:?}",
            steps[1]
        );
        let Step::Effect(EffectStep::ForEach { steps, .. }) =
            &move_steps(&game, "STONE", 0, WHITE)[2]
        else {
            panic!("the stone's third step is a FOR_EACH");
        };
        assert!(
            matches!(&steps[0], Step::If { then, .. }
                if matches!(&then[0], Step::Effect(EffectStep::SetOwner(at)) if at.anchor == Anchor::Each)),
            "inside FOR_EACH, SET_OWNER is at EACH: {:?}",
            steps[0]
        );

        // Judged: BOARD counts every piece of the board, and EACH is the place a FOR_EACH is at.
        let spec = load_spec();
        let mut labels = Labels::default();
        let mut build = |test: Value| {
            let test: TestSpec = serde_json::from_value(test).expect("a test");
            Test::build(&test, &spec.players, None, &mut labels)
        };
        let two_rooks = build(json!({
            "test": "COUNT", "in": { "region": "BOARD" }, "where": { "codes": ["ROOK"] }, "min": 2
        }));
        let enemy_at_each = build(json!({
            "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "EACH" } },
            "where": { "owner": "ENEMY" }
        }));
        let mut game = load_game();
        let at = |x: u8, y: u8| Position::from_coordinates(&[x, y]);
        game.state
            .pieces
            .insert(at(0, 0), Piece::new("ROOK", WHITE));
        let target = at(3, 3);
        let judged = |game: &Game, test: &Test, each: Option<&Position>| {
            judge(
                test,
                &TestContext {
                    game,
                    pieces: crate::logic::PiecesView::of(&game.state.pieces),
                    player: WHITE,
                    start: None,
                    target: &target,
                    each,
                    removed: &[],
                    moved: &[],
                    after_turn: None,
                    last: None,
                },
            )
        };
        assert!(!judged(&game, &two_rooks, None), "one rook on the board");
        game.state
            .pieces
            .insert(at(7, 7), Piece::new("ROOK", BLACK));
        assert!(judged(&game, &two_rooks, None), "two rooks, far apart");
        assert!(
            judged(&game, &enemy_at_each, Some(&at(7, 7))),
            "EACH on BLACK's rook"
        );
        assert!(
            !judged(&game, &enemy_at_each, Some(&at(0, 0))),
            "EACH on an ally"
        );
        assert!(
            !judged(&game, &enemy_at_each, None),
            "no EACH outside FOR_EACH"
        );
    }

    #[test]
    fn a_misspelled_field_in_a_step_is_refused() {
        let chose = one_action(json!([{ "chose": "PIECE", "where": {} }]));
        let form = one_action(json!([
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            { "effect": "MOVE", "form": { "place": "TARGET" }, "to": { "place": "TARGET" } }
        ]));
        let doo = small_spec(json!([{ "name": "A", "doo": [] }]));
        for (field, spec) in [
            ("`chose`", chose),
            ("unknown field `form`", form),
            ("unknown field `doo`", doo),
        ] {
            let error = read(&spec)
                .err()
                .unwrap_or_else(|| panic!("{field} should be refused"));
            assert!(
                error.to_string().contains(field),
                "the refusal names {field}: {error}"
            );
        }
    }

    #[test]
    fn a_step_with_two_kinds_or_none_is_refused() {
        let both = one_action(json!([{
            "test": "COUNT",
            "in": { "region": "BOARD" },
            "effect": "END_TURN"
        }]));
        let none = one_action(json!([{}]));
        for (named, spec) in [("has `test`, `effect`", both), ("has none", none)] {
            let error = read(&spec)
                .err()
                .unwrap_or_else(|| panic!("the step that {named} should be refused"));
            assert!(
                error
                    .to_string()
                    .contains("exactly one of `choose`, `test`, `effect` and `if`")
                    && error.to_string().contains(named),
                "the refusal says the step {named}: {error}"
            );
        }
    }

    #[test]
    fn refusals_when_the_spec_is_read() {
        let piece = json!({ "choose": "PIECE", "where": { "owner": "ALLY" } });
        let board = json!({ "choose": "PLACE", "in": { "region": "BOARD" } });
        let at = |place: &str| json!({ "place": place });

        let mut move_inside_a_move = with_a_pass();
        move_inside_a_move["pieces"][0]["moves"][0]["do"] = json!([{ "choose": "MOVE" }]);

        // Inside a piece's move only a CODE choice is allowed: any other would set START or
        // TARGET again.
        let in_a_move = |choice: Value| {
            let mut spec = with_a_pass();
            spec["pieces"][0]["moves"][0]["do"] = json!([choice, move_effect()]);
            spec
        };

        // A test step is refused anywhere inside a FOR_EACH, in an `if`'s branch too.
        let test_in_an_if_in_for_each = one_action(json!([
            piece,
            {
                "effect": "FOR_EACH",
                "in": { "region": "AROUND", "of": at("START") },
                "do": [{
                    "if": { "test": "COUNT", "in": { "region": "AT", "at": at("EACH") } },
                    "then": [
                        { "test": "COUNT", "in": { "region": "AT", "at": at("EACH") } },
                        { "effect": "REMOVE", "at": at("EACH") }
                    ]
                }]
            }
        ]));

        let bare_test_in_for_each = one_action(json!([
            piece,
            {
                "effect": "FOR_EACH",
                "in": { "region": "AROUND", "of": at("START") },
                "do": [
                    { "test": "COUNT", "in": { "region": "AT", "at": at("EACH") } },
                    { "effect": "REMOVE", "at": at("EACH") }
                ]
            }
        ]));

        let path_in_an_action = one_action(json!([
            piece,
            board,
            { "test": "COUNT", "in": { "region": "PATH", "from": at("START"), "to": at("TARGET") }, "max": 0, "min": 0 }
        ]));

        let cases: Vec<(&str, &str, Value)> = vec![
            (
                "PIECE",
                "action A",
                one_action(json!([{ "choose": "MOVE" }])),
            ),
            (
                "PIECE",
                "action A",
                one_action(
                    json!([{ "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" } }, { "choose": "MOVE" }]),
                ),
            ),
            (
                "TARGET",
                "action A",
                one_action(json!([
                    { "test": "COUNT", "in": { "region": "AT", "at": at("TARGET") } }
                ])),
            ),
            (
                "TARGET",
                "action A",
                one_action(json!([
                    { "if": { "test": "COUNT", "in": { "region": "BOARD" } }, "then": [board] },
                    { "effect": "REMOVE", "at": at("TARGET") }
                ])),
            ),
            (
                "EACH",
                "action A",
                one_action(json!([piece, { "effect": "REMOVE", "at": at("EACH") }])),
            ),
            (
                "promo",
                "action A",
                one_action(json!([
                    piece,
                    { "effect": "SET_CODE", "at": at("START"), "code": { "chosen": "promo" } }
                ])),
            ),
            (
                "DRAGON",
                "action A",
                one_action(
                    json!([board, { "effect": "PLACE", "at": at("TARGET"), "code": "DRAGON" }]),
                ),
            ),
            (
                "WYVERN",
                "action A",
                one_action(
                    json!([piece, { "effect": "SET_CODE", "at": at("START"), "code": "WYVERN" }]),
                ),
            ),
            (
                "GRIFFIN",
                "action A",
                one_action(
                    json!([{ "choose": "CODE", "as": "x", "options": ["PROBE", "GRIFFIN"] }]),
                ),
            ),
            (
                "FOR_EACH",
                "action A",
                one_action(json!([
                    piece,
                    { "effect": "FOR_EACH", "in": { "region": "AROUND", "of": at("START") }, "do": [board] }
                ])),
            ),
            (
                "TWIN",
                "TWIN",
                small_spec(json!([{ "name": "TWIN", "do": [] }, { "name": "TWIN", "do": [] }])),
            ),
            (
                "GREEN",
                "action A",
                small_spec(json!([{ "name": "A", "players": ["GREEN"], "do": [] }])),
            ),
            ("MOVE choice", "move 0 of piece PROBE", move_inside_a_move),
            (
                "PIECE choice",
                "move 0 of piece PROBE",
                in_a_move(piece.clone()),
            ),
            (
                "PLACE choice",
                "move 0 of piece PROBE",
                in_a_move(board.clone()),
            ),
            (
                "HELD choice",
                "move 0 of piece PROBE",
                in_a_move(json!({ "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" } })),
            ),
            ("test inside a FOR_EACH", "action A", bare_test_in_for_each),
            (
                "test inside a FOR_EACH",
                "action A",
                test_in_an_if_in_for_each,
            ),
            ("PATH", "action A", path_in_an_action),
        ];
        for (named, where_, spec) in cases {
            let error = read(&spec)
                .err()
                .unwrap_or_else(|| panic!("the spec naming {named} in {where_} should be refused"));
            assert!(
                !matches!(error, GameSpecError::JsonError(_)),
                "{named}: refused while checking, not while parsing: {error}"
            );
            assert!(
                error.to_string().contains(named),
                "the refusal names {named}: {error}"
            );
            assert!(
                error.to_string().contains(where_),
                "the refusal of {named} says where ({where_}): {error}"
            );
        }

        // A place both branches of an `if` set is set after it.
        let both_branches = one_action(json!([
            { "if": { "test": "COUNT", "in": { "region": "BOARD" } }, "then": [board], "else": [board] },
            { "effect": "REMOVE", "at": at("TARGET") }
        ]));
        assert!(
            read(&both_branches).is_ok(),
            "{:?}",
            read(&both_branches).err()
        );
    }

    #[test]
    fn a_game_needs_actions_and_a_move_needs_do() {
        let mut no_actions = small_spec(json!([{ "name": "A", "do": [] }]));
        no_actions
            .as_object_mut()
            .expect("an object")
            .remove("actions");
        let mut empty_actions = small_spec(json!([]));
        let mut no_do = small_spec(json!([{ "name": "A", "do": [] }]));
        no_do["pieces"][0]["moves"][0]
            .as_object_mut()
            .expect("an object")
            .remove("do");
        for (what, spec) in [
            ("a spec with no `actions`", no_actions),
            ("a spec with an empty `actions`", empty_actions.clone()),
            ("a move with no `do`", no_do),
        ] {
            assert!(read(&spec).is_err(), "{what} is refused");
        }
        empty_actions["actions"] = json!([{ "name": "A", "do": [] }]);
        assert!(read(&empty_actions).is_ok());
    }

    #[test]
    fn start_after_a_hand_choice_is_allowed_in_tests() {
        let hand_piece_is_a_probe = json!({
            "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "START" } },
            "where": { "codes": ["PROBE"] }
        });
        let drop = one_action(json!([
            { "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" }, "where": { "codes": ["PROBE"] } },
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            hand_piece_is_a_probe,
            move_effect()
        ]));
        assert!(read(&drop).is_ok(), "{:?}", read(&drop).err());

        let no_hand = one_action(json!([
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            hand_piece_is_a_probe
        ]));
        let error = read(&no_hand).expect_err("START is set by no choice");
        assert!(
            matches!(&error, GameSpecError::InvalidStep(why) if why.contains("START")),
            "{error}"
        );

        let owner_in_hand = one_action(
            json!([{ "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" }, "where": { "owner": "ALLY" } }]),
        );
        let error = read(&owner_in_hand).expect_err("a hand's filter takes codes only");
        assert!(
            matches!(&error, GameSpecError::InvalidStep(why) if why.contains("codes")),
            "{error}"
        );
    }

    /// Whether move `id` of piece `code` threatens, as compiled.
    fn threatens(game: &Game, code: &str, id: u8) -> bool {
        game.rules.blueprints[&crate::shared::Symbol::new(code)]
            .move_blueprints
            .iter()
            .find(|movement| movement.id == id)
            .expect("the move is there")
            .threatens
    }

    #[test]
    fn a_move_that_requires_an_empty_target_does_not_threaten() {
        let game = load_game();
        for (code, id, how) in [
            ("PAWN", 0, "NOT COUNT at TARGET"),
            ("PAWN", 1, "COUNT at TARGET with min 0 and max 0, in an ALL"),
            ("PAWN", 3, "en passant: NOT COUNT at TARGET"),
            ("MAN", 0, "a jump onto an empty square"),
            ("KING", 1, "castling"),
            ("STONE", 0, "a copy onto an empty square"),
        ] {
            assert!(
                !threatens(&game, code, id),
                "{code} move {id} ({how}) threatens"
            );
        }

        let tests = parse_game_spec("./src/tests/test_blocks/spec.json")
            .map(Game::from_spec)
            .expect("the test_blocks spec is read");
        assert!(
            !threatens(&tests, "SEER", 0),
            "SEER moves onto empty squares only"
        );
    }

    #[test]
    fn a_move_that_allows_an_enemy_target_threatens() {
        let game = load_game();
        for (code, id) in [
            ("ROOK", 0),
            ("PAWN", 2),
            ("KING", 0),
            ("SILVER", 0),
            ("BOMB", 0),
        ] {
            assert!(threatens(&game, code, id), "{code} move {id} threatens");
        }

        // A test that the target is empty counts only before the first effect, only on its own
        // or in a top-level ALL, and only at TARGET itself.
        let empty = json!({ "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "min": 0, "max": 0 });
        for steps in [
            json!([move_effect(), empty]),
            json!([{ "test": "ANY", "of": [empty] }, move_effect()]),
            json!([
                { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET", "offset": [0, 1] } } } },
                move_effect()
            ]),
        ] {
            let mut spec = with_a_pass();
            spec["pieces"][0]["moves"][0]["do"] = steps.clone();
            let game = Game::from_spec(read(&spec).expect("the spec is read"));
            assert!(threatens(&game, "PROBE", 0), "{steps} threatens");
        }

        let tests = parse_game_spec("./src/tests/test_blocks/spec.json")
            .map(Game::from_spec)
            .expect("the test_blocks spec is read");
        assert!(
            threatens(&tests, "ROOK", 0),
            "a move onto a square no ally holds threatens"
        );
    }

    #[test]
    fn removes_beyond_target_is_noted_for_an_atomic_like_capture() {
        assert!(
            load_game().rules.removes_beyond_target,
            "BOMB removes START and the places around TARGET"
        );

        let mut without_bomb = spec_value();
        without_bomb["pieces"]
            .as_array_mut()
            .expect("pieces")
            .retain(|piece| piece["code"] != "BOMB");
        let game = Game::from_spec(read(&without_bomb).expect("the spec is read"));
        assert!(
            !game.rules.removes_beyond_target,
            "en passant removes at an offset from START, and the stone's FOR_EACH removes nothing"
        );

        let at_start = one_action(json!([
            { "choose": "PIECE", "where": {} },
            { "effect": "REMOVE", "at": { "place": "START" } }
        ]));
        let game = Game::from_spec(read(&at_start).expect("the spec is read"));
        assert!(
            game.rules.removes_beyond_target,
            "an action removes at START"
        );

        let tests = parse_game_spec("./src/tests/test_blocks/spec.json")
            .map(Game::from_spec)
            .expect("the test_blocks spec is read");
        assert!(!tests.rules.removes_beyond_target);
    }
}
