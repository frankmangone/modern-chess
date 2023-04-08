mod piece;
mod board;

// use crate::board::{ Board, Position };
use crate::piece::{
    // Piece, 
    movements::{ 
        Movement as Mov, 
        Direction as Dir, 
        Steps as Stp,
        Action as Act
    }
};
use std::fs::{ read_to_string };
use serde_json;
use parity_scale_codec::{ Encode, Decode };

fn main() {
    // ENCODING:
    // ----------------------------------------------------------------
    //

    // println!("Pawn:");
    // println!("{:?}", Mov { action: Act::Move, positions: [Dir::Player(Stp::Pos(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::InitialMove, positions: [Dir::Player(Stp::Pos(2)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Player(Stp::Pos(1)), Dir::PlayerOrth(Stp::Pos(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Player(Stp::Pos(1)), Dir::PlayerOrth(Stp::Neg(1))] }.encode());

    // println!("--------------------------------");

    // println!("Rook:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Pos(0)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Neg(0)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(0)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(0)), Dir::None] }.encode());

    // println!("--------------------------------");

    // println!("Knight:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(1)), Dir::Hor(Stp::Pos(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(2)), Dir::Hor(Stp::Pos(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(1)), Dir::Hor(Stp::Pos(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(2)), Dir::Hor(Stp::Pos(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(1)), Dir::Hor(Stp::Neg(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(2)), Dir::Hor(Stp::Neg(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(1)), Dir::Hor(Stp::Neg(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(2)), Dir::Hor(Stp::Neg(1))] }.encode());

    // println!("--------------------------------");

    // println!("Bishop:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(0)), Dir::Hor(Stp::Pos(0))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(0)), Dir::Hor(Stp::Neg(0))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(0)), Dir::Hor(Stp::Pos(0))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(0)), Dir::Hor(Stp::Neg(0))] }.encode());

    // println!("--------------------------------");

    // println!("Queen:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Pos(0)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Neg(0)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(0)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(0)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(0)), Dir::Hor(Stp::Pos(0))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(0)), Dir::Hor(Stp::Neg(0))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(0)), Dir::Hor(Stp::Pos(0))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(0)), Dir::Hor(Stp::Neg(0))] }.encode());

    // println!("--------------------------------");

    // println!("King:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(1)), Dir::Hor(Stp::Pos(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(1)), Dir::Hor(Stp::Pos(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(1)), Dir::Hor(Stp::Neg(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(1)), Dir::Hor(Stp::Neg(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Pos(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Neg(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Pos(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Neg(1)), Dir::None] }.encode());

    // DECODING:
    // ----------------------------------------------------------------

    let ser_json = read_to_string("movements.json").unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&ser_json).expect("JSON was not well-formatted");

    let pawn = Mov::deserialize(json.get("pawn").unwrap().clone()).unwrap();
    let rook = Mov::deserialize(json.get("rook").unwrap().clone()).unwrap();
    let knight = Mov::deserialize(json.get("knight").unwrap().clone()).unwrap();
    let bishop = Mov::deserialize(json.get("bishop").unwrap().clone()).unwrap();
    let queen = Mov::deserialize(json.get("queen").unwrap().clone()).unwrap();
    let king = Mov::deserialize(json.get("king").unwrap().clone()).unwrap();

    println!("Pawn movements: {:?}", pawn);
    println!("Rook movements: {:?}", rook);
    println!("Knight movements: {:?}", knight);
    println!("Bishop movements: {:?}", bishop);
    println!("Queen movements: {:?}", queen);
    println!("King movements: {:?}", king);
}

// CHESS BOARD:
// ----------------------------------------------------------------

// let mut board = Board::new(8, 8);

// let white_pawn = Piece::new("pawn", 0);
// let white_rook = Piece::new("rook", 0);
// let white_knight = Piece::new("knight", 0);
// let white_bishop = Piece::new("bishop", 0);
// let white_queen = Piece::new("queen", 0);
// let white_king = Piece::new("king", 0);

// let black_pawn = Piece::new("pawn", 1);
// let black_rook = Piece::new("rook", 1);
// let black_knight = Piece::new("knight", 1);
// let black_bishop = Piece::new("bishop", 1);
// let black_queen = Piece::new("queen", 1);
// let black_king = Piece::new("king", 1);

// // Set up a chess board
// board.add_piece(&Position(0,0), &white_rook).ok();
// board.add_piece(&Position(0,1), &white_pawn).ok();
// board.add_piece(&Position(0,6), &black_pawn).ok();
// board.add_piece(&Position(0,6), &black_rook).ok();

// board.add_piece(&Position(1,0), &white_knight).ok();
// board.add_piece(&Position(1,1), &white_pawn).ok();
// board.add_piece(&Position(1,6), &black_pawn).ok();
// board.add_piece(&Position(1,6), &black_knight).ok();

// board.add_piece(&Position(2,0), &white_bishop).ok();
// board.add_piece(&Position(2,1), &white_pawn).ok();
// board.add_piece(&Position(2,6), &black_pawn).ok();
// board.add_piece(&Position(2,6), &black_bishop).ok();

// board.add_piece(&Position(3,0), &white_queen).ok();
// board.add_piece(&Position(3,1), &white_pawn).ok();
// board.add_piece(&Position(3,6), &black_pawn).ok();
// board.add_piece(&Position(3,6), &black_queen).ok();

// board.add_piece(&Position(4,0), &white_king).ok();
// board.add_piece(&Position(4,1), &white_pawn).ok();
// board.add_piece(&Position(4,6), &black_pawn).ok();
// board.add_piece(&Position(4,6), &black_king).ok();

// board.add_piece(&Position(5,0), &white_bishop).ok();
// board.add_piece(&Position(5,1), &white_pawn).ok();
// board.add_piece(&Position(5,6), &black_pawn).ok();
// board.add_piece(&Position(5,6), &black_bishop).ok();

// board.add_piece(&Position(6,0), &white_knight).ok();
// board.add_piece(&Position(6,1), &white_pawn).ok();
// board.add_piece(&Position(6,6), &black_pawn).ok();
// board.add_piece(&Position(6,6), &black_knight).ok();

// board.add_piece(&Position(7,0), &white_rook).ok();
// board.add_piece(&Position(7,1), &white_pawn).ok();
// board.add_piece(&Position(7,6), &black_pawn).ok();
// board.add_piece(&Position(7,6), &black_rook).ok();

// println!("This is how a chess board would look: {:?}", board.pieces);
