//! Building a game from its spec.

use std::collections::HashMap;
use std::sync::Arc;

use crate::logic::pieces::piece::Piece;
use crate::logic::pieces::PieceBlueprint;
use crate::logic::places::board::{graph::Labels, Board};
use crate::logic::places::holders::{self, Holder, HolderInfo, HolderOwner};
use crate::logic::rules::ends;
use crate::logic::rules::steps::{self, Action};
use crate::logic::rules::test_block::Test;
use crate::logic::{Game, GamePhase, GameState, Rules, StageRule, Zone};
use crate::shared::{IdMap, Position, Symbol};
use crate::specs::{GameSpec, Visible};

/// Whether any object inside `value` has a `moved` key holding a bool: a filter that asks
/// whether a piece has moved.
fn reads_moved(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.get("moved").is_some_and(serde_json::Value::is_boolean)
                || map.values().any(reads_moved)
        }
        serde_json::Value::Array(items) => items.iter().any(reads_moved),
        _ => false,
    }
}

impl Game {
    pub fn from_spec(spec: GameSpec) -> Self {
        let moved_matters = serde_json::to_value(&spec)
            .map(|value| reads_moved(&value))
            .unwrap_or(true);
        // Process turn information.
        let turn_order = spec.turns.order;
        let current_turn = spec.turns.start_at;

        // Create blueprints for each piece & player.
        // TODO: Optimize for pieces that are not direction-dependent.
        let mut blueprints: IdMap<Symbol, PieceBlueprint> = IdMap::default();
        let mut captured_as: IdMap<Symbol, Option<Symbol>> = IdMap::default();

        let mut piece_values: HashMap<String, u32> = HashMap::new();

        for piece_spec in &spec.pieces {
            captured_as.insert(
                Symbol::new(&piece_spec.code),
                piece_spec.captured_as.as_deref().map(Symbol::new),
            );
            if let Some(value) = piece_spec.value {
                piece_values.insert(piece_spec.code.clone(), value);
            }
        }
        // The blueprints collect every edge label their rules follow; the board graph is then
        // compiled with an edge table for each.
        let mut labels = Labels::default();
        for piece_spec in spec.pieces {
            blueprints.insert(
                Symbol::new(&piece_spec.code),
                PieceBlueprint::from_spec(piece_spec.clone(), spec.players.clone(), &mut labels),
            );
        }
        let actions: Vec<Action> = spec
            .actions
            .iter()
            .map(|action| Action::build(action, &spec.every_action, &spec.players, &mut labels))
            .collect();
        let must: Vec<Test> = spec
            .must
            .iter()
            .map(|test| Test::build(test, &spec.players, None, &mut labels))
            .collect();
        let stages: Vec<StageRule> = spec
            .turns
            .stages
            .iter()
            .map(|stage| {
                let only = stage
                    .only
                    .as_ref()
                    .map(|test| Test::build(test, &spec.players, None, &mut labels));
                StageRule {
                    name: Symbol::new(&stage.name),
                    only,
                    players: stage.players.clone().unwrap_or_default(),
                    has_actions: actions
                        .iter()
                        .any(|action| action.stage == Some(Symbol::new(&stage.name))),
                }
            })
            .collect();
        let end_checks: Vec<ends::EndCheck> = spec
            .end_checks
            .iter()
            .map(|check| ends::EndCheck::build(check, &spec.players, &mut labels))
            .collect();
        let score: Vec<ends::ScoreTerm> = spec.score.iter().map(ends::ScoreTerm::from).collect();
        let move_steps = blueprints
            .values()
            .flat_map(|blueprint: &PieceBlueprint| &blueprint.move_blueprints)
            .map(|movement| &movement.steps);
        let removes_beyond_target = actions
            .iter()
            .map(|action| &action.steps)
            .chain(move_steps)
            .flat_map(|by_player| by_player.values())
            .any(|steps| steps::removes_beyond_target(steps));
        let chance = actions
            .iter()
            .flat_map(|action| action.steps.values())
            .any(|steps| steps::uses_chance(steps));
        let map_holders: Vec<(String, u32)> = spec
            .board
            .map
            .iter()
            .flat_map(|map| map.places.iter())
            .filter(|place| place.capacity.unwrap_or(1) > 1)
            .map(|place| (place.name.clone(), place.capacity.unwrap_or(1)))
            .collect();
        let map_starts: Vec<(String, std::collections::BTreeMap<String, u32>)> = spec
            .board
            .map
            .iter()
            .flat_map(|map| map.places.iter())
            .filter(|place| !place.start.is_empty())
            .map(|place| (place.name.clone(), place.start.clone()))
            .collect();
        let board = Board::new(spec.board, &labels);

        // Process player information.
        let mut players: Vec<String> = Vec::new();
        let mut pieces: IdMap<Position, Piece> = IdMap::default();

        for player in spec.players.into_iter() {
            // Store players' names (identifiers).
            players.push(player.name.clone());

            // Add pieces to the board, based on the starting positions for each player.
            for starting_positions in player.starting_positions {
                let piece_code = starting_positions.piece;

                for position in starting_positions.positions {
                    pieces.insert(
                        position.position(),
                        Piece::new(piece_code.clone(), player.name.clone()),
                    );
                }
            }
        }

        // What the bags and the places that hold many are, and keep from the start.
        let mut holder_rules: IdMap<Position, HolderInfo> = IdMap::default();
        for bag in &spec.bags {
            if bag.for_each_player() {
                for player in &players {
                    holder_rules.insert(
                        holders::bag_position(&bag.name, Some(player)),
                        HolderInfo {
                            ordered: bag.ordered,
                            capacity: bag.capacity,
                            owner: HolderOwner::Player(Symbol::new(player)),
                            visible: bag.visible,
                            shows_count: bag.shows_count(),
                        },
                    );
                }
            } else {
                holder_rules.insert(
                    holders::bag_position(&bag.name, None),
                    HolderInfo {
                        ordered: bag.ordered,
                        capacity: bag.capacity,
                        owner: HolderOwner::Shared,
                        visible: bag.visible,
                        shows_count: bag.shows_count(),
                    },
                );
            }
        }
        for (name, capacity) in &map_holders {
            holder_rules.insert(
                Position::new(name),
                HolderInfo {
                    ordered: false,
                    capacity: Some(*capacity),
                    owner: HolderOwner::Place,
                    visible: Visible::All,
                    shows_count: true,
                },
            );
        }
        let mut holders: IdMap<Position, Holder> = IdMap::default();
        for bag in &spec.bags {
            let owners: Vec<Option<&str>> = if bag.for_each_player() {
                players.iter().map(|player| Some(player.as_str())).collect()
            } else {
                vec![None]
            };
            for owner in owners {
                for (code, count) in &bag.start.for_owner(owner) {
                    holders
                        .entry(holders::bag_position(&bag.name, owner))
                        .or_default()
                        .add(Symbol::new(owner.unwrap_or("")), Symbol::new(code), *count);
                }
            }
        }
        for (name, start) in &map_starts {
            for (code, count) in start {
                holders.entry(Position::new(name)).or_default().add(
                    Symbol::new(""),
                    Symbol::new(code),
                    *count,
                );
            }
        }

        let zones: HashMap<String, Zone> = spec
            .zones
            .into_iter()
            .map(|zone| {
                let places = zone
                    .places
                    .into_iter()
                    .map(|(player, places)| {
                        let names = places.iter().map(|p| p.position().to_string()).collect();
                        (player, names)
                    })
                    .collect();
                (zone.name, Zone { places })
            })
            .collect();

        let mut game = Game {
            rules: Arc::new(Rules {
                turn_order: turn_order.clone(),
                name: spec.name,
                zones,
                players,
                hand_enabled: spec.bags.iter().any(|bag| bag.name == "HAND"),
                moved_matters,
                stages,
                actions,
                must,
                end_checks,
                score,
                removes_beyond_target,
                chance,
                holders: holder_rules,
                captured_as,
                piece_values,
                blueprints,
            }),
            board,
            state: GameState {
                turn_order,
                pieces,
                current_turn,
                ways: None,
                phase: GamePhase::Choosing { chosen: Vec::new() },
                stage: None,
                asked: Vec::new(),
                together: None,
                focus: None,
                ask_then: None,
                history: Vec::new(),
                position_hashes: Vec::new(),
                holders,
                seed: None,
                draws: 0,
                saved_hand: HashMap::new(),
                counters: HashMap::new(),
                captured: HashMap::new(),
            },
        };
        game.work_out_ways();
        game
    }
}
