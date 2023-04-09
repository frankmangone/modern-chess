mod board;
mod piece;

use crate::board::{
    position::Position,
    presets::setup_chess_board,
};
use crate::piece::{
    movements::{Action as Act, Direction as Dir, Movement as Mov, Steps as Stp},
    Piece,
};
use parity_scale_codec::{ Encode };

fn main() {
    // ENCODING:
    // ----------------------------------------------------------------
    //

    // println!("Pawn:");
    // println!("{:?}", Mov { action: Act::Move, positions: [Dir::Player(Stp::Value(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::InitialMove, positions: [Dir::Player(Stp::Value(2)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Player(Stp::Value(1)), Dir::PlayerOrth(Stp::Value(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Player(Stp::Value(1)), Dir::PlayerOrth(Stp::Value(-1))] }.encode());

    // println!("--------------------------------");

    // println!("Rook:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(-1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Every(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Every(-1)), Dir::None] }.encode());

    // println!("--------------------------------");

    // println!("Knight:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(1)), Dir::Hor(Stp::Value(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(2)), Dir::Hor(Stp::Value(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(1)), Dir::Hor(Stp::Value(-2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(2)), Dir::Hor(Stp::Value(-1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(-1)), Dir::Hor(Stp::Value(2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(-2)), Dir::Hor(Stp::Value(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(-1)), Dir::Hor(Stp::Value(-2))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(-2)), Dir::Hor(Stp::Value(-1))] }.encode());

    // println!("--------------------------------");

    // println!("Bishop:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(1)), Dir::Hor(Stp::Every(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(1)), Dir::Hor(Stp::Every(-1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(-1)), Dir::Hor(Stp::Every(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(-1)), Dir::Hor(Stp::Every(-1))] }.encode());

    // println!("--------------------------------");

    // println!("Queen:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(-1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Every(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Every(-1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(1)), Dir::Hor(Stp::Every(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(1)), Dir::Hor(Stp::Every(-1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(-1)), Dir::Hor(Stp::Every(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Every(-1)), Dir::Hor(Stp::Every(-1))] }.encode());

    // println!("--------------------------------");

    // println!("King:");
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(1)), Dir::Hor(Stp::Value(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(-1)), Dir::Hor(Stp::Value(1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(1)), Dir::Hor(Stp::Value(-1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(-1)), Dir::Hor(Stp::Value(-1))] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Value(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Hor(Stp::Value(-1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(1)), Dir::None] }.encode());
    // println!("{:?}", Mov { action: Act::Capture, positions: [Dir::Ver(Stp::Value(-1)), Dir::None] }.encode());

    // DECODING:
    // ----------------------------------------------------------------

    let mut board = setup_chess_board().unwrap();
    dbg!(&board);

    board.find_movements(&Position::new(5, 5)).ok();

    dbg!(board.available_movements);
}
