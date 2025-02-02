use std::io::{self, Write};
use modern_chess::specs::parse_game_spec;
use modern_chess::logic::Game;

use modern_chess::specs::GameSpecError;

fn main() -> Result<(), GameSpecError> {
    // Load chess specification
    let game_spec = parse_game_spec("specs/chess.json")?;
    // mut
    let game = Game::from_spec(game_spec);
    
    play_game(game);

    Ok(())
}

fn play_game(game: Game) {
    // loop {
        print_board(&game);
        
    //     let current_player = game.current_turn.clone();
    //     println!("Current player: {}", current_player);
        
    //     if let Some(position) = get_piece_selection() {
    //         let moves = game.board.borrow().calculate_moves(&current_player, &position);
        
    //         match moves {
    //             Some(valid_moves) => {
    //                 println!("Valid moves: {:?}", valid_moves);
                    
    //                 if let Some(target) = get_move_selection() {
    //                     if valid_moves.contains(&target) {
    //                         game.borrow_mut().make_move(&position, &target);
    //                     } else {
    //                         println!("Invalid move!");
    //                         continue;
    //                     }
    //                 }
    //             }
    //             None => {
    //                 println!("No valid moves for this piece!");
    //                 continue;
    //             }
    //         }
    //     }
    // }
}

fn print_board(game: &Game) {
    let board = game.board.borrow();
    
    println!("---------------------------------");

    for j in 0..board.dimensions[1] {
        let mut str = String::from("|");

        for i in (0..board.dimensions[0]).rev() {
            let position = vec![i, j];

            match board.piece_at_position(&position) {
                Some(piece) => str.push_str(&piece.code[..3]),
                None => str.push_str("..."),
            }
            
            str.push_str("|");
        }
        println!("{}", str);
        
        println!("---------------------------------");
    }
}

// fn get_piece_selection() -> Option<Position> {
//     print!("Select piece (e.g., 'e2'): ");
//     io::stdout().flush().unwrap();
    
//     let mut input = String::new();
//     io::stdin().read_line(&mut input).unwrap();
//     parse_position(&input.trim())
// }

// fn get_move_selection() -> Option<Position> {
//     print!("Select destination (e.g., 'e4'): ");
//     io::stdout().flush().unwrap();
    
//     let mut input = String::new();
//     io::stdin().read_line(&mut input).unwrap();
//     parse_position(&input.trim())
// }

// fn parse_position(input: &str) -> Option<Position> {
//     if input.len() != 2 {
//         return None;
//     }
    
//     let file = input.chars().nth(0)? as i16 - 'a' as i16;
//     let rank = input.chars().nth(1)?.to_digit(10)? as i16 - 1;
    
//     if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
//         Some(Position(vec![rank, file]))
//     } else {
//         None
//     }
// } 