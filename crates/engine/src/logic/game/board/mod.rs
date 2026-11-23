use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use crate::shared::{Coordinates, Position};
use crate::specs::{BoardShape, BoardSpec, MapSpec};

pub mod graph;
pub mod map;

use graph::{Graph, LabelId, Labels, NodeId};

/// A `Board` is a representation of everything board-related. Of course,
/// boards contain pieces, and have a shape that establishes which positions
/// are viable.
///
/// The engine walks the board as a graph (see `Graph`): a grid or map spec is compiled into one
/// when the game is built. The grid's description is kept for drawing it; a map's places are
/// kept in the graph (see `places`).
#[derive(Clone, Debug)]
pub struct Board {
    // Board shape specifications
    pub dimensions: Vec<u8>,
    pub disabled_positions: HashSet<Coordinates>,
    /// How the board's cells meet: square or hex.
    pub shape: BoardShape,
    graph: Arc<Graph>,
}

// ---------------------------------------------------------------------
// Associated fns to parse spec
// ---------------------------------------------------------------------
impl Board {
    /// A board with an edge table for every label in `labels`.
    pub fn new(board_spec: BoardSpec, labels: &Labels) -> Board {
        let graph = Arc::new(match &board_spec.map {
            Some(map) => Graph::map(map, labels),
            None => Graph::grid(&board_spec, labels),
        });
        Board {
            dimensions: board_spec.dimensions,
            disabled_positions: board_spec.disabled_positions,
            shape: board_spec.shape,
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

    /// Every position on the board, in board order, without copying them.
    pub fn positions(&self) -> &[Position] {
        self.graph.names()
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
                for neighbour in self.adjacent(node) {
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

    /// The node `position` names; `None` when it is not on the board. Nodes are numbered in
    /// board order (for a grid: lexicographic, first coordinate outermost).
    pub fn node(&self, position: &Position) -> Option<NodeId> {
        self.graph.node(position)
    }

    pub fn name(&self, node: NodeId) -> Option<&Position> {
        self.graph.name(node)
    }

    pub fn follow(&self, node: NodeId, label: LabelId) -> Option<NodeId> {
        self.graph.follow(node, label)
    }

    /// The nodes `label` links `node` to on a map, in board order. Empty on a grid.
    pub fn linked(&self, node: NodeId, label: LabelId) -> &[NodeId] {
        self.graph.linked(node, label)
    }

    /// The map's places, kinds and links; `None` on a grid.
    pub fn places(&self) -> Option<&MapSpec> {
        self.graph.places()
    }

    /// The position one edge labelled `label` away from `position`.
    pub fn step(&self, position: &Position, label: LabelId) -> Option<&Position> {
        let node = self.follow(self.node(position)?, label)?;
        self.name(node)
    }

    pub fn path(&self, from: NodeId, to: NodeId, unit: LabelId) -> Option<Vec<NodeId>> {
        self.graph.path(from, to, unit)
    }

    pub fn orthogonal(&self, node: NodeId) -> Vec<NodeId> {
        self.graph.orthogonal(node)
    }

    pub fn adjacent(&self, node: NodeId) -> Vec<NodeId> {
        self.graph.adjacent(node)
    }

    /// The grid coordinates of `position`, when it is a grid square (never on a map).
    pub fn coordinates(&self, position: &Position) -> Option<&[u8]> {
        self.graph.coordinates(self.node(position)?)
    }
}
