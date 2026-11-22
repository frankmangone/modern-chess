//! The boards embedded in the docs. They live in `apps/docs/src/boards/`, where
//! `apps/docs/build.sh` (or `node apps/docs/scripts/derive-boards.mjs` plus the copies) puts them.
mod support;

use modern_chess_session::session::Session;
use modern_chess_session::view::Status;
use std::path::PathBuf;
use support::{at, destinations, play};

fn boards_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/docs/src/boards")
}

fn session(name: &str) -> Session {
    let path = boards_dir().join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("{} is missing: run apps/docs/build.sh (or node apps/docs/scripts/derive-boards.mjs) first", path.display())
    });
    Session::new(&text).unwrap_or_else(|e| panic!("{name} does not load: {e}"))
}

fn has_piece(s: &Session, x: u8, y: u8) -> Option<(String, String)> {
    s.view()
        .pieces
        .iter()
        .find(|p| p.position == format!("{x},{y}"))
        .map(|p| (p.code.clone(), p.player.clone()))
}

#[test]
fn every_board_loads_and_has_a_playable_move() {
    let mut seen = 0;
    for entry in std::fs::read_dir(boards_dir()).expect("boards dir") {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        if !file.ends_with(".json") || file.ends_with(".skin.json") {
            continue;
        }
        let name = file.trim_end_matches(".json");
        let mut s = session(name);
        let view = s.view();
        // The first player can choose something: a piece that moves, a place, or a hand piece.
        let choice = &view.choice;
        let movable = choice.places.iter().any(|place| {
            at(&mut s, place.name.as_str()).is_ok() && !s.view().choice.places.is_empty()
        }) || !choice.hand.is_empty()
            || !choice.actions.is_empty();
        assert!(movable, "{name}: the first player has no legal move");
        seen += 1;
    }
    assert!(seen >= 6, "expected the shipped boards, found {seen}");
}

#[test]
fn special_moves_offers_both_castles() {
    let mut s = session("special_moves");
    at(&mut s, "4,0").unwrap();
    let dests = destinations(&s.view());
    assert!(
        dests.contains(&"6,0".to_string()),
        "king-side castle missing: {dests:?}"
    );
    assert!(
        dests.contains(&"2,0".to_string()),
        "queen-side castle missing: {dests:?}"
    );
    at(&mut s, "6,0").unwrap();
    assert_eq!(has_piece(&s, 6, 0).map(|p| p.0), Some("KING".into()));
    assert_eq!(
        has_piece(&s, 5, 0).map(|p| p.0),
        Some("ROOK".into()),
        "the rook must hop over"
    );
    assert_eq!(has_piece(&s, 7, 0), None);
}

#[test]
fn special_moves_pawn_can_push_two() {
    let mut s = session("special_moves");
    at(&mut s, "6,1").unwrap();
    assert_eq!(
        destinations(&s.view()),
        vec!["6,2".to_string(), "6,3".to_string()]
    );
}

#[test]
fn special_moves_allows_en_passant() {
    let mut s = session("special_moves");
    play(&mut s, "6,1", "6,2"); // White waits
    play(&mut s, "3,6", "3,4"); // Black pushes two, landing beside White's e5 pawn
    at(&mut s, "4,4").unwrap();
    assert!(
        destinations(&s.view()).contains(&"3,5".to_string()),
        "en passant square missing"
    );
    at(&mut s, "3,5").unwrap();
    assert_eq!(has_piece(&s, 3, 5).map(|p| p.1), Some("WHITE".into()));
    assert_eq!(has_piece(&s, 3, 4), None, "the passed pawn must be removed");
}

#[test]
fn hill_is_won_by_stepping_onto_the_centre() {
    let mut s = session("hill");
    at(&mut s, "3,2").unwrap();
    assert!(destinations(&s.view()).contains(&"3,3".to_string()));
    at(&mut s, "3,3").unwrap();
    assert_eq!(
        s.view().status,
        Status::Won {
            winner: "WHITE".into()
        }
    );
}

#[test]
fn stalemate_puzzle_ends_in_a_draw() {
    let mut s = session("stalemate");
    at(&mut s, "2,4").unwrap();
    assert!(destinations(&s.view()).contains(&"2,6".to_string()));
    at(&mut s, "2,6").unwrap();
    assert_eq!(s.view().status, Status::Draw);
}

#[test]
fn atomic_blast_removes_the_cluster_but_not_the_pawn() {
    let mut s = session("atomic_blast");
    play(&mut s, "0,0", "0,4");
    for (x, y) in [(0, 0), (0, 4), (1, 4), (1, 3)] {
        assert_eq!(has_piece(&s, x, y), None, "({x},{y}) should be gone");
    }
    assert_eq!(has_piece(&s, 0, 5), Some(("PAWN".into(), "BLACK".into())));
    assert!(
        has_piece(&s, 7, 0).is_some() && has_piece(&s, 7, 7).is_some(),
        "kings survive"
    );
}

#[test]
fn promotion_board_promotes_to_the_only_option_without_asking() {
    let mut s = session("promotion");
    play(&mut s, "2,3", "2,4");
    assert!(s.view().choice.codes.is_empty());
    assert_eq!(
        has_piece(&s, 2, 4),
        Some(("MANTRI".to_string(), "WHITE".to_string()))
    );
}
