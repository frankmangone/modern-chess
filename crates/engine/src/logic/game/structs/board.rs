use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use crate::shared::{Coordinates, ExtendedPosition, Position};
use crate::specs::BoardSpec;

use super::graph::{Graph, LabelId, Labels, NodeId};

/// A `Board` is a representation of everything board-related. Of course,
/// boards contain pieces, and have a shape that establishes which positions
/// are viable.
///
/// The engine walks the board as a graph (see `Graph`): a grid spec is compiled into one when
/// the game is built. The grid's description is kept for drawing it.
#[derive(Clone, Debug)]
pub struct Board {
    // Board shape specifications
    pub dimensions: Vec<u8>,
    pub disabled_positions: HashSet<Coordinates>,
    graph: Arc<Graph>,
}

// ---------------------------------------------------------------------
// Associated fns to parse spec
// ---------------------------------------------------------------------
impl Board {
    /// A board with an edge table for every label in `labels`.
    pub fn new(board_spec: BoardSpec, labels: &Labels) -> Board {
        let graph = Arc::new(Graph::grid(&board_spec, labels));
        Board {
            dimensions: board_spec.dimensions,
            disabled_positions: board_spec.disabled_positions,
            graph,
        }
    }

    /// A board with no edge labels: its positions and neighbourhoods only.
    pub fn from_spec(board_spec: BoardSpec) -> Board {
        Board::new(board_spec, &Labels::default())
    }
}

// ---------------------------------------------------------------------
// Logic-related associated fns
// ---------------------------------------------------------------------
impl Board {
    /// Returns every position on the board, in board order (for a grid: lexicographic, first
    /// coordinate outermost).
    pub fn all_positions(&self) -> Vec<Position> {
        self.graph.names().to_vec()
    }

    /// Returns every position within `radius` rounds of `adjacent` from `center`, `center`
    /// included, in board order. Empty when `center` is not on the board.
    pub fn positions_within(&self, center: &Position, radius: u8) -> Vec<Position> {
        let Some(start) = self.node(center) else {
            return vec![];
        };
        let mut reached: HashSet<NodeId> = HashSet::from([start]);
        let mut frontier: VecDeque<NodeId> = VecDeque::from([start]);
        for _ in 0..radius {
            let mut next = VecDeque::new();
            for node in frontier {
                for &neighbour in self.adjacent(node) {
                    if reached.insert(neighbour) {
                        next.push_back(neighbour);
                    }
                }
            }
            frontier = next;
        }
        let mut nodes: Vec<NodeId> = reached.into_iter().collect();
        nodes.sort_unstable();
        nodes
            .into_iter()
            .filter_map(|node| self.name(node).cloned())
            .collect()
    }

    /// Whether `position` names a square of the board.
    pub fn is_position_valid(&self, position: &ExtendedPosition) -> bool {
        let square: Option<Coordinates> = position.iter().map(|&v| u8::try_from(v).ok()).collect();
        square.is_some_and(|square| self.node(&Position::from_coordinates(&square)).is_some())
    }

    pub fn node(&self, position: &Position) -> Option<NodeId> {
        self.graph.node(position)
    }

    pub fn name(&self, node: NodeId) -> Option<&Position> {
        self.graph.name(node)
    }

    pub fn follow(&self, node: NodeId, label: LabelId) -> Option<NodeId> {
        self.graph.follow(node, label)
    }

    /// The position one edge labelled `label` away from `position`.
    pub fn step(&self, position: &Position, label: LabelId) -> Option<&Position> {
        let node = self.follow(self.node(position)?, label)?;
        self.name(node)
    }

    pub fn path(&self, from: NodeId, to: NodeId, unit: LabelId) -> Option<Vec<NodeId>> {
        self.graph.path(from, to, unit)
    }

    pub fn orthogonal(&self, node: NodeId) -> &[NodeId] {
        self.graph.orthogonal(node)
    }

    pub fn adjacent(&self, node: NodeId) -> &[NodeId] {
        self.graph.adjacent(node)
    }

    /// The grid coordinates of `position`, when it is a grid square.
    pub fn coordinates(&self, position: &Position) -> Option<&[u8]> {
        self.graph.coordinates(self.node(position)?)
    }

    /// Where `position` comes in board order; `None` when it is not on the board.
    pub fn order(&self, position: &Position) -> Option<NodeId> {
        self.node(position)
    }
}
