//! The boards embedded in the docs. They live in `apps/docs/src/boards/`, where
//! `apps/docs/build.sh` (or `node apps/docs/scripts/derive-boards.mjs` plus the copies) puts them.
use modern_chess_session::session::Session;
use modern_chess_session::view::Status;
use std::path::PathBuf;

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
        .find(|p| p.x == x && p.y == y)
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
        let movable = view
            .pieces
            .iter()
            .filter(|p| p.player == view.turn)
            .any(|p| s.select(p.x, p.y).is_ok() && !s.view().destinations.is_empty());
        // A game of placing pieces (Go) has drops instead of moves.
        let can_place = view.drops.values().any(|squares| !squares.is_empty());
        assert!(
            movable || can_place,
            "{name}: the first player has no legal move"
        );
        seen += 1;
    }
    assert!(seen >= 6, "expected the shipped boards, found {seen}");
}

#[test]
fn special_moves_offers_both_castles() {
    let mut s = session("special_moves");
    s.select(4, 0).unwrap();
    let dests = s.view().destinations;
    assert!(
        dests.contains(&[6, 0]),
        "king-side castle missing: {dests:?}"
    );
    assert!(
        dests.contains(&[2, 0]),
        "queen-side castle missing: {dests:?}"
    );
    s.make_move(6, 0).unwrap();
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
    s.select(6, 1).unwrap();
    assert_eq!(s.view().destinations, vec![[6, 2], [6, 3]]);
}

#[test]
fn special_moves_allows_en_passant() {
    let mut s = session("special_moves");
    s.select(6, 1).unwrap();
    s.make_move(6, 2).unwrap(); // White waits
    s.select(3, 6).unwrap();
    s.make_move(3, 4).unwrap(); // Black pushes two, landing beside White's e5 pawn
    s.select(4, 4).unwrap();
    assert!(
        s.view().destinations.contains(&[3, 5]),
        "en passant square missing"
    );
    s.make_move(3, 5).unwrap();
    assert_eq!(has_piece(&s, 3, 5).map(|p| p.1), Some("WHITE".into()));
    assert_eq!(has_piece(&s, 3, 4), None, "the passed pawn must be removed");
}

#[test]
fn hill_is_won_by_stepping_onto_the_centre() {
    let mut s = session("hill");
    s.select(3, 2).unwrap();
    assert!(s.view().destinations.contains(&[3, 3]));
    s.make_move(3, 3).unwrap();
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
    s.select(2, 4).unwrap();
    assert!(s.view().destinations.contains(&[2, 6]));
    s.make_move(2, 6).unwrap();
    assert_eq!(s.view().status, Status::Draw);
}

#[test]
fn atomic_blast_removes_the_cluster_but_not_the_pawn() {
    let mut s = session("atomic_blast");
    s.select(0, 0).unwrap();
    s.make_move(0, 4).unwrap();
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
fn promotion_board_asks_what_to_promote_to() {
    let mut s = session("promotion");
    s.select(2, 3).unwrap();
    s.make_move(2, 4).unwrap();
    assert_eq!(s.view().promotion, Some(vec!["MANTRI".to_string()]));
}
