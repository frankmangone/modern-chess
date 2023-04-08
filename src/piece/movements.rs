use parity_scale_codec::{ Decode };
use parity_scale_codec_derive::{ Encode, Decode };
use serde_json::Value;

#[derive(Encode, Decode)]
#[derive(Debug)]
pub enum Steps {
  Pos(u8), // Pos(0) means "infinitely" in the positive direction
  Neg(u8)  // Neg(0) means "infinitely" in the negative direction
}

/// An enum representing the possible directions of movement
/// `Ver` and `Hor` are straightforward;
/// `Player` is more complicated, but think that each player fill have a direction that
/// should be "forward" - that is what's represented by `Player`. `PlayerOrth` is the orthogonal
/// to said direction.
#[derive(Encode, Decode)]
#[derive(Debug)]
pub enum Direction {
  None,
  Ver(Steps),
  Hor(Steps),
  Player(Steps),
  PlayerOrth(Steps),
}

#[derive(Encode, Decode)]
#[derive(Debug)]
pub enum Action {
  Capture,
  Move,
  InitialMove,
  CaptureEnPassant,
  // Let's leave some space for "special" actions!
}

#[derive(Encode, Decode)]
#[derive(Debug)]
pub struct Movement {
  pub action: Action,
  pub positions: [Direction; 2]
}

impl Movement {  
  pub fn deserialize(value: Value) -> Result<Vec<Movement>, ()> {
    // We expect an array of arrays (for now at least!), with the outer array being a list
    // of possible movements, while the inner ones are encoded movements
    match value {
      Value::Array(movements) => Self::decode_movements(movements),
      _ => Err(()),
    }
  }

  fn decode_movements(movements: Vec<Value>) -> Result<Vec<Movement>, ()> {
    movements.into_iter()
        .map(|movement| match movement {
          Value::Array(value) => {
            Self::decode_movement(value)
          },
          _ => Err(()),
        })
        .collect()
  }

  fn decode_movement(movement: Vec<Value>) -> Result<Movement, ()> {
    let vectorized: Result<Vec<u8>, ()> = movement.into_iter()
        .map(|movement_value| match movement_value {
          Value::Number(value) => {
            match value.as_u64() {
              Some(number) => Ok(number as u8),
              None => Err(())
            }
          },
          _ => Err(()),
        })
        .collect();

    match vectorized {
      Err(_) => Err(()),
      Ok(vec) => {
        match Self::decode(&mut &vec[..]) {
          Ok(decoded) => Ok(decoded),
          Err(_) => Err(())
        }
      },
    }
  }
}
