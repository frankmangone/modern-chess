use std::io::{self, Write};
use modern_chess::specs::parse_game_spec;
use modern_chess::logic::{Game, GamePhase, GameTransition};
use modern_chess::shared::Position;
use modern_chess::specs::GameSpecError;

fn main() -> Result<(), GameSpecError> {
    // Load chess specification
    let game_spec = parse_game_spec("specs/chess.json")?;
    let mut game = Game::from_spec(game_spec);

    play_game(&mut game);

    Ok(())
}

fn play_game(game: &mut Game) -> () {
    loop {
        match &game.state.phase {
            GamePhase::Idle => {
                print_board(&game);
        
                let current_player = game.current_player();
                println!("Current player: {}", current_player);

                if let Some(position) = get_piece_selection() {
                    game.transition(GameTransition::CalculateMoves{ position }).unwrap_or_else(|err| println!("Error: {}", err));
                }
            },
            GamePhase::Moving { position: _ } => {
                println!("Valid moves:");

                // This is safe to unwrap because we know the phase is Moving.
                let valid_moves = &game.state.available_moves.as_ref().unwrap();

                for valid_move in valid_moves.keys() {
                    println!("{:?}: {:?}", valid_move, valid_moves.get(valid_move).unwrap().action);
                }

                if let Some(target) = get_move_selection() {
                    game.transition(GameTransition::ExecuteMove{ position: target }).unwrap_or_else(|err| println!("Error: {}", err));
                }
            }
            GamePhase::Transforming { position: _pos, options: _ } => {
                // TODO: Implement transformation
            }
        }
    }
}

fn print_board(game: &Game) {
    let board = &game.board;
    
    println!("---------------------------------");

    for j in (0..board.dimensions[1]).rev() {
        let mut str = String::from("|");

        for i in 0..board.dimensions[0] {
            let position = vec![i, j];

            match game.piece_at_position(&position) {
                Some(piece) => {
                    // Color pieces based on player
                    let colored_code = if piece.player == "WHITE" {
                        format!("\x1b[34m{}\x1b[0m", &piece.code[..3]) // Blue for player 0
                    } else {
                        format!("\x1b[31m{}\x1b[0m", &piece.code[..3]) // Red for player 1
                    };
                    str.push_str(&colored_code);
                },
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