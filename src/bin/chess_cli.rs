use std::io::{self, Write};
use modern_chess::specs::parse_game_spec;
use modern_chess::logic::Game;
use modern_chess::shared::Position;
use modern_chess::specs::GameSpecError;

fn main() -> Result<(), GameSpecError> {
    // Load chess specification
    let game_spec = parse_game_spec("specs/chess.json")?;
    // mut
    let mut game = Game::from_spec(game_spec);
    
    play_game(&mut game);

    Ok(())
}

fn play_game(game: &mut Game) {
    loop {
        print_board(&game);
        
        let current_player = game.current_player();
        println!("Current player: {}", current_player);
        
        if let Some(position) = get_piece_selection() {
            game.calculate_moves(position);
        
            match &game.available_moves {
                Some(valid_moves) => {
                    println!("Valid moves:");

                    for valid_move in valid_moves.keys() {
                        println!("{:?}: {:?}", valid_move, valid_moves.get(valid_move).unwrap().action);
                    }
                    
                    if let Some(target) = get_move_selection() {
                        if valid_moves.contains_key(&target) {
                            println!("Executing move: {:?}", target);
                            game.execute_move(target);
                        } else {
                            println!("Invalid move!");
                        }
                    }
                }
                None => {
                    println!("No valid moves for this piece!");
                }
            }
        }
    }
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

fn get_piece_selection() -> Option<Position> {
    print!("Select position to see available moves (e.g., [0, 1]): ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    parse_position(&input.trim())
}

fn get_move_selection() -> Option<Position> {
    print!("Select move to execute (e.g., [0, 1]): ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    parse_position(&input.trim())
}

// FIXME: This does not account for invalid inputs.
fn parse_position(input: &str) -> Option<Position> {
    let trimmed = input.trim_matches(|c| c == '[' || c == ']'); // Remove brackets
    let position: Vec<u8> = trimmed
        .split(',')
        .filter_map(|s| s.trim().parse::<u8>().ok()) // Split and parse to u8
        .collect(); // Collect into a Vec<u8>

    Some(position)
} 