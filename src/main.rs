mod board;
mod piece;

use crate::board::{Board, Position};
use crate::piece::{
    movements::{Action as Act, Direction as Dir, Movement as Mov, Steps as Stp},
    Piece,
};
use parity_scale_codec::{Decode, Encode};
use serde_json;
use std::collections::HashMap;
use std::fs::read_to_string;

fn main() {
    // ENCODING:
    // ----------------------------------------------------------------
    //

    // println!("Pawn:");
    // println!("{:?}", Mov { action: Act::Move, positions: [Dir::Player(Stp::PosValue(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::InitialMove, positions: [Dir::Player(Stp::PosValue(2)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Player(Stp::PosValue(1)), Dir::PlayerOrth(Stp::PosValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Player(Stp::PosValue(1)), Dir::PlayerOrth(Stp::NegValue(1))] }.encode());

    // println!("--------------------------------");

    // println!("Rook:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::PosEvery(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::NegEvery(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosEvery(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegEvery(1)), Dir::None] }.encode());

    // println!("--------------------------------");

    // println!("Knight:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosValue(1)), Dir::Hor(Stp::PosValue(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosValue(2)), Dir::Hor(Stp::PosValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosValue(1)), Dir::Hor(Stp::NegValue(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosValue(2)), Dir::Hor(Stp::NegValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegValue(1)), Dir::Hor(Stp::PosValue(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegValue(2)), Dir::Hor(Stp::PosValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegValue(1)), Dir::Hor(Stp::NegValue(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegValue(2)), Dir::Hor(Stp::NegValue(1))] }.encode());

    // println!("--------------------------------");

    // println!("Bishop:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosEvery(1)), Dir::Hor(Stp::PosEvery(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosEvery(1)), Dir::Hor(Stp::NegEvery(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegEvery(1)), Dir::Hor(Stp::PosEvery(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegEvery(1)), Dir::Hor(Stp::NegEvery(1))] }.encode());

    // println!("--------------------------------");

    // println!("Queen:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::PosEvery(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::NegEvery(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosEvery(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegEvery(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosEvery(1)), Dir::Hor(Stp::PosEvery(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosEvery(1)), Dir::Hor(Stp::NegEvery(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegEvery(1)), Dir::Hor(Stp::PosEvery(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegEvery(1)), Dir::Hor(Stp::NegEvery(1))] }.encode());

    // println!("--------------------------------");

    // println!("King:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosValue(1)), Dir::Hor(Stp::PosValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegValue(1)), Dir::Hor(Stp::PosValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosValue(1)), Dir::Hor(Stp::NegValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegValue(1)), Dir::Hor(Stp::NegValue(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::PosValue(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::NegValue(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::PosValue(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::NegValue(1)), Dir::None] }.encode());

    // DECODING:
    // ----------------------------------------------------------------

    let board = setup_chess().unwrap();
    dbg!(board.pieces);
}

fn load_pieces() -> HashMap<String, Piece> {
    let ser_json = read_to_string("movements.json").unwrap();
    let json: HashMap<String, serde_json::Value> =
        serde_json::from_str(&ser_json).expect("JSON was not well-formatted");

    let mut pieces: HashMap<String, Piece> = HashMap::new();

    for (str, val) in &json {
        let symbol = String::from(str);
        let encoded_movements = val.clone();
        let movements = Mov::deserialize(encoded_movements).unwrap();
        pieces.insert(
            symbol.clone(),
            Piece {
                symbol: symbol.clone(),
                player: 0,
                movements,
            },
        );
    }

    pieces
}

// CHESS BOARD:
// ----------------------------------------------------------------

fn setup_chess() -> Result<Board, ()> {
    let mut board = Board::new(8, 8);

    let pieces = load_pieces();

    let pawn: &Piece = pieces.get("pawn").unwrap();
    let rook = pieces.get("rook").unwrap();
    let knight = pieces.get("knight").unwrap();
    let bishop = pieces.get("bishop").unwrap();
    let queen = pieces.get("queen").unwrap();
    let king = pieces.get("king").unwrap();

    let white_pawn = Piece::with_team(pawn, 0);
    let white_rook = Piece::with_team(rook, 0);
    let white_knight = Piece::with_team(knight, 0);
    let white_bishop = Piece::with_team(bishop, 0);
    let white_queen = Piece::with_team(queen, 0);
    let white_king = Piece::with_team(king, 0);

    let black_pawn = Piece::with_team(pawn, 1);
    let black_rook = Piece::with_team(rook, 1);
    let black_knight = Piece::with_team(knight, 1);
    let black_bishop = Piece::with_team(bishop, 1);
    let black_queen = Piece::with_team(queen, 1);
    let black_king = Piece::with_team(king, 1);

    // Set up a chess board
    board.add_piece(&Position(0, 0), &white_rook).ok();
    board.add_piece(&Position(0, 1), &white_pawn).ok();
    board.add_piece(&Position(0, 6), &black_pawn).ok();
    board.add_piece(&Position(0, 6), &black_rook).ok();

    board.add_piece(&Position(1, 0), &white_knight).ok();
    board.add_piece(&Position(1, 1), &white_pawn).ok();
    board.add_piece(&Position(1, 6), &black_pawn).ok();
    board.add_piece(&Position(1, 6), &black_knight).ok();

    board.add_piece(&Position(2, 0), &white_bishop).ok();
    board.add_piece(&Position(2, 1), &white_pawn).ok();
    board.add_piece(&Position(2, 6), &black_pawn).ok();
    board.add_piece(&Position(2, 6), &black_bishop).ok();

    board.add_piece(&Position(3, 0), &white_queen).ok();
    board.add_piece(&Position(3, 1), &white_pawn).ok();
    board.add_piece(&Position(3, 6), &black_pawn).ok();
    board.add_piece(&Position(3, 6), &black_queen).ok();

    board.add_piece(&Position(4, 0), &white_king).ok();
    board.add_piece(&Position(4, 1), &white_pawn).ok();
    board.add_piece(&Position(4, 6), &black_pawn).ok();
    board.add_piece(&Position(4, 6), &black_king).ok();

    board.add_piece(&Position(5, 0), &white_bishop).ok();
    board.add_piece(&Position(5, 1), &white_pawn).ok();
    board.add_piece(&Position(5, 6), &black_pawn).ok();
    board.add_piece(&Position(5, 6), &black_bishop).ok();

    board.add_piece(&Position(6, 0), &white_knight).ok();
    board.add_piece(&Position(6, 1), &white_pawn).ok();
    board.add_piece(&Position(6, 6), &black_pawn).ok();
    board.add_piece(&Position(6, 6), &black_knight).ok();

    board.add_piece(&Position(7, 0), &white_rook).ok();
    board.add_piece(&Position(7, 1), &white_pawn).ok();
    board.add_piece(&Position(7, 6), &black_pawn).ok();
    board.add_piece(&Position(7, 6), &black_rook).ok();

    Ok(board)
}
