use std::io::{self, Write};
use std::env;
use modern_chess::specs::parse_game_spec;
use modern_chess::logic::{Game, GamePhase, GameTransition};
use modern_chess::shared::Position;
use modern_chess::specs::GameSpecError;

// One ANSI color per player slot (supports up to 6 players).
const PLAYER_COLORS: &[&str] = &[
    "\x1b[34m", // blue
    "\x1b[31m", // red
    "\x1b[33m", // yellow
    "\x1b[35m", // magenta
    "\x1b[32m", // green
    "\x1b[36m", // cyan
];
const RESET: &str = "\x1b[0m";

fn main() -> Result<(), GameSpecError> {
    let spec_path = env::args().nth(1).unwrap_or_else(|| "specs/chess.json".to_string());
    println!("Loading spec: {}", spec_path);

    let game_spec = parse_game_spec(&spec_path)?;
    let mut game = Game::from_spec(game_spec);

    play_game(&mut game);

    Ok(())
}

fn play_game(game: &mut Game) {
    loop {
        match &game.state.phase {
            GamePhase::Idle => {
                print_board(&game);
        
                let current_player = game.current_player();
                println!("Current player: {}", current_player);

                if let Some(position) = get_piece_selection() {
                    game.transition(GameTransition::CalculateMoves{ position }).unwrap_or_else(|err| println!("Error: {:?}", err));
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
                    game.transition(GameTransition::ExecuteMove{ position: target }).unwrap_or_else(|err| println!("Error: {:?}", err));
                }
            }
            GamePhase::Transforming { position: _pos, options } => {
                if let Some(option) = get_option_selection(options.clone()) {
                    game.transition(GameTransition::Transform{ target: option }).unwrap_or_else(|err| println!("Error: {:?}", err));
                }
            }
        }
    }
}

fn print_board(game: &Game) {
    let board = &game.board;
    let cols = board.dimensions[0] as usize;
    let rows = board.dimensions[1] as usize;

    // Each cell is "|XXX" (4 chars) plus a trailing "|", so total = cols*4+1.
    let separator = "-".repeat(cols * 4 + 1);

    // Row labels are right-aligned into 2 chars + 1 space = 3 char prefix.
    // Separator lines use 3 spaces so they stay flush with the row content.
    let sep_prefix = "   ";

    // Column header: 5 leading spaces put the first digit over the cell centre,
    // then each column number left-padded to 4 chars to match cell width.
    let col_header = format!(
        "     {}",
        (0..cols).map(|i| format!("{:<4}", i)).collect::<String>()
    );

    // Print legend
    let legend: Vec<String> = game.players.iter().enumerate().map(|(i, name)| {
        let color = PLAYER_COLORS[i % PLAYER_COLORS.len()];
        format!("{}[{}]{}", color, name, RESET)
    }).collect();
    println!("Players: {}", legend.join("  "));

    println!("{}", col_header);
    println!("{}{}", sep_prefix, separator);

    for j in (0..rows).rev() {
        let row_prefix = format!("{:>2} ", j);
        let mut row = String::from("|");

        for i in 0..cols {
            let position = vec![i as u8, j as u8];

            match game.piece_at_position(&position) {
                Some(piece) => {
                    let player_index = game.players.iter().position(|p| p == &piece.player).unwrap_or(0);
                    let color = PLAYER_COLORS[player_index % PLAYER_COLORS.len()];
                    row.push_str(&format!("{}{}{}", color, &piece.code[..3], RESET));
                },
                None => row.push_str("..."),
            }

            row.push('|');
        }
        println!("{}{}", row_prefix, row);
        println!("{}{}", sep_prefix, separator);
    }

    println!("{}", col_header);
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

fn get_option_selection(options: Vec<String>) -> Option<String> {
    print!("Select option from {:?}: ", options);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    Some(input.trim().to_string())
}

fn parse_position(input: &str) -> Option<Position> {
    let trimmed = input.trim_matches(|c| c == '[' || c == ']');
    let parts: Vec<&str> = trimmed.split(',').collect();
    // Reject blank input.
    if parts.len() == 1 && parts[0].trim().is_empty() {
        return None;
    }
    // All parts must parse successfully; any failure returns None.
    parts.iter()
        .map(|s| s.trim().parse::<u8>().ok())
        .collect()
}