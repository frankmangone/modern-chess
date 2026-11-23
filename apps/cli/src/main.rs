use modern_chess::logic::{Answer, Choice, Game, GamePhase, GameTransition};
use modern_chess::shared::Position;
use modern_chess::specs::parse_game_spec;
use modern_chess::specs::GameSpecError;
use std::env;
use std::io::{self, Write};

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

const DEFAULT_SPEC: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/engine/specs/chess.json"
);

fn main() -> Result<(), GameSpecError> {
    let spec_path = env::args().nth(1).unwrap_or(DEFAULT_SPEC.to_string());
    println!("Loading spec: {}", spec_path);

    let game_spec = parse_game_spec(&spec_path)?;
    let mut game = Game::from_spec(game_spec);

    play_game(&mut game);

    Ok(())
}

fn play_game(game: &mut Game) {
    loop {
        if let GamePhase::GameOver { winner } = &game.state.phase {
            print_board(game);
            match winner {
                Some(player) => println!("Checkmate! {} wins!", player),
                None => println!("Stalemate! It's a draw."),
            }
            break;
        }

        print_board(game);
        println!("Current player: {}", game.current_player());
        let choice = game.choice();
        print_choice(&choice);

        let transition = match read_input(&choice) {
            Some(Input::Answer(answer)) => GameTransition::Choose(answer),
            Some(Input::Cancel) => GameTransition::Cancel,
            Some(Input::End) => break,
            None => continue,
        };
        game.transition(transition)
            .unwrap_or_else(|err| println!("Error: {:?}", err));
    }
}

/// What the player can answer next.
fn print_choice(choice: &Choice) {
    if !choice.chosen.is_empty() {
        println!("Chosen so far: {:?}", choice.chosen);
    }
    if let Some(start) = &choice.start {
        println!("Selected: {}", start);
    }
    if !choice.places.is_empty() {
        let places: Vec<String> = choice
            .places
            .iter()
            .map(|place| format!("{} ({:?})", place.name, place.r#for))
            .collect();
        println!("Places: {}", places.join(", "));
    }
    if !choice.codes.is_empty() {
        println!("Codes: {}", choice.codes.join(", "));
    }
    if !choice.held.is_empty() {
        println!("Held: {}", choice.held.join(", "));
    }
    if !choice.actions.is_empty() {
        let actions: Vec<String> = choice
            .actions
            .iter()
            .map(|action| match action.plays {
                true => format!("{} (plays)", action.name),
                false => action.name.clone(),
            })
            .collect();
        println!("Actions: {}", actions.join(", "));
    }
}

/// One line typed by the player.
enum Input {
    Answer(Answer),
    Cancel,
    /// The input ended.
    End,
}

/// Reads one answer: a place (`4,1` or `[4, 1]`, or a map place's name), a code, a piece in
/// a bag or an action's name among those offered, or `cancel`.
fn read_input(choice: &Choice) -> Option<Input> {
    print!("Answer (a place, a code, a held piece, an action, or cancel): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).unwrap() == 0 {
        return Some(Input::End);
    }
    parse_input(input.trim(), choice)
}

fn parse_input(input: &str, choice: &Choice) -> Option<Input> {
    if input.is_empty() {
        return None;
    }
    if input == "cancel" {
        return Some(Input::Cancel);
    }
    let input_is = |name: &String| name == input;
    let answer = if choice.codes.iter().any(input_is) {
        Answer::Code(input.to_string())
    } else if choice.held.iter().any(input_is) {
        Answer::Held(input.to_string())
    } else if choice.actions.iter().any(|action| input_is(&action.name)) {
        Answer::Action(input.to_string())
    } else {
        Answer::Place(parse_position(input).unwrap_or_else(|| Position::from(input)))
    };
    Some(Input::Answer(answer))
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
    let legend: Vec<String> = game
        .rules
        .players
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let color = PLAYER_COLORS[i % PLAYER_COLORS.len()];
            format!("{}[{}]{}", color, name, RESET)
        })
        .collect();
    println!("Players: {}", legend.join("  "));

    println!("{}", col_header);
    println!("{}{}", sep_prefix, separator);

    for j in (0..rows).rev() {
        let row_prefix = format!("{:>2} ", j);
        let mut row = String::from("|");

        for i in 0..cols {
            let position = Position::from_coordinates(&[i as u8, j as u8]);

            match game.piece_at_position(&position) {
                Some(piece) => {
                    let player_index = game
                        .rules
                        .players
                        .iter()
                        .position(|p| p == &piece.player)
                        .unwrap_or(0);
                    let color = PLAYER_COLORS[player_index % PLAYER_COLORS.len()];
                    row.push_str(&format!("{}{}{}", color, &piece.code[..3], RESET));
                }
                None => row.push_str("..."),
            }

            row.push('|');
        }
        println!("{}{}", row_prefix, row);
        println!("{}{}", sep_prefix, separator);
    }

    println!("{}", col_header);
}

fn parse_position(input: &str) -> Option<Position> {
    let trimmed = input.trim_matches(|c| c == '[' || c == ']');
    let parts: Vec<&str> = trimmed.split(',').collect();
    // Reject blank input.
    if parts.len() == 1 && parts[0].trim().is_empty() {
        return None;
    }
    // All parts must parse successfully; any failure returns None.
    let coordinates: Option<Vec<u8>> = parts.iter().map(|s| s.trim().parse::<u8>().ok()).collect();
    coordinates.map(|c| Position::from_coordinates(&c))
}
