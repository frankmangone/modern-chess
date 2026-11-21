use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::shared::{apply_direction, Direction, ExtendedPosition, Position};
use crate::specs::BoardSpec;

/// A node of the board graph: an index into the graph's tables.
pub type NodeId = u32;

/// An edge label of the board graph: an index into the graph's edge tables.
pub type LabelId = u32;

/// Marks "no edge" in an edge table.
const NO_NODE: NodeId = NodeId::MAX;

/// What a rule follows to get from one position to another: the label of the move itself, and
/// the label of one step along the same line, which is what a path between the two walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offset {
    pub label: LabelId,
    pub unit: LabelId,
}

/// Every edge label the rules of a game use. A grid's label is the vector it moves by. The
/// blueprints collect them while they are built, and the board then builds one edge table per
/// label, so it never builds a table that no rule follows.
#[derive(Clone, Debug, Default)]
pub struct Labels {
    vectors: Vec<ExtendedPosition>,
    index: HashMap<ExtendedPosition, LabelId>,
}

impl Labels {
    /// The labels of `vector` and of its unit vector (each component reduced to its sign).
    pub fn intern(&mut self, vector: &ExtendedPosition) -> Offset {
        let unit: ExtendedPosition = vector.iter().map(|value| value.signum()).collect();
        Offset {
            label: self.label(vector),
            unit: self.label(&unit),
        }
    }

    /// A player's label for a vector of the spec. On a grid, the player's label map is their
    /// orientation: the vector is turned by it before it is looked up.
    pub fn for_player(&mut self, direction: &Direction, vector: &ExtendedPosition) -> Offset {
        self.intern(&apply_direction(direction, vector))
    }

    /// The vectors of every label, indexed by `LabelId`.
    pub fn vectors(&self) -> &[ExtendedPosition] {
        &self.vectors
    }

    fn label(&mut self, vector: &ExtendedPosition) -> LabelId {
        if let Some(&label) = self.index.get(vector) {
            return label;
        }
        let label = LabelId::try_from(self.vectors.len()).unwrap_or(LabelId::MAX);
        self.vectors.push(vector.clone());
        self.index.insert(vector.clone(), label);
        label
    }
}

/// The board as the engine walks it: named nodes, one edge table per label, and the
/// neighbourhoods some rules look at (built the first time one is asked for).
#[derive(Debug)]
pub struct Graph {
    names: Vec<Position>,
    index: HashMap<Position, NodeId>,
    coordinates: Vec<Vec<u8>>,
    edges: Vec<Vec<NodeId>>,
    orthogonal: OnceLock<Vec<Vec<NodeId>>>,
    adjacent: OnceLock<Vec<Vec<NodeId>>>,
}

impl Graph {
    /// Compiles a grid into a graph. Squares become nodes named by their coordinates, numbered
    /// in lexicographic order (first coordinate outermost) with disabled squares left out. Each
    /// label gets an edge from every square to the square its vector reaches.
    pub fn grid(spec: &BoardSpec, labels: &Labels) -> Graph {
        let coordinates = grid_squares(&spec.dimensions, &spec.disabled_positions);
        let names: Vec<Position> = coordinates.iter().map(|c| grid_name(c)).collect();
        let index = names
            .iter()
            .enumerate()
            .filter_map(|(node, name)| Some((name.clone(), NodeId::try_from(node).ok()?)))
            .collect();
        let mut graph = Graph {
            names,
            index,
            coordinates,
            edges: Vec::new(),
            orthogonal: OnceLock::new(),
            adjacent: OnceLock::new(),
        };
        graph.edges = labels
            .vectors()
            .iter()
            .map(|vector| {
                graph
                    .coordinates
                    .iter()
                    .map(|square| graph.offset_square(square, vector).unwrap_or(NO_NODE))
                    .collect()
            })
            .collect();
        graph
    }

    pub fn names(&self) -> &[Position] {
        &self.names
    }

    pub fn node(&self, name: &Position) -> Option<NodeId> {
        self.index.get(name).copied()
    }

    pub fn name(&self, node: NodeId) -> Option<&Position> {
        self.names.get(usize::try_from(node).ok()?)
    }

    pub fn coordinates(&self, node: NodeId) -> Option<&[u8]> {
        self.coordinates
            .get(usize::try_from(node).ok()?)
            .map(Vec::as_slice)
    }

    pub fn follow(&self, node: NodeId, label: LabelId) -> Option<NodeId> {
        let table = self.edges.get(usize::try_from(label).ok()?)?;
        let next = *table.get(usize::try_from(node).ok()?)?;
        (next != NO_NODE).then_some(next)
    }

    /// The nodes after `from` up to and including `to`, walking `unit`. Empty when `from` is
    /// `to`; `None` when the walk leaves the board, or goes on longer than the board is big,
    /// without reaching `to`.
    pub fn path(&self, from: NodeId, to: NodeId, unit: LabelId) -> Option<Vec<NodeId>> {
        let mut nodes = Vec::new();
        let mut current = from;
        for _ in 0..self.names.len() {
            if current == to {
                return Some(nodes);
            }
            current = self.follow(current, unit)?;
            nodes.push(current);
        }
        (current == to).then_some(nodes)
    }

    /// The squares one step along a single axis, in the order axis 0 back, axis 0 forward,
    /// axis 1 back, and so on.
    pub fn orthogonal(&self, node: NodeId) -> &[NodeId] {
        let lists = self.orthogonal.get_or_init(|| {
            let axes = self.coordinates.first().map_or(0, Vec::len);
            let mut vectors = Vec::new();
            for axis in 0..axes {
                for delta in [-1i16, 1] {
                    let mut vector = vec![0i16; axes];
                    if let Some(value) = vector.get_mut(axis) {
                        *value = delta;
                    }
                    vectors.push(vector);
                }
            }
            self.neighbours_along(&vectors)
        });
        neighbours_of(lists, node)
    }

    /// The squares within one step along every axis.
    pub fn adjacent(&self, node: NodeId) -> &[NodeId] {
        let lists = self.adjacent.get_or_init(|| {
            let axes = self.coordinates.first().map_or(0, Vec::len);
            let mut vectors: Vec<ExtendedPosition> = vec![Vec::new()];
            for _ in 0..axes {
                vectors = vectors
                    .into_iter()
                    .flat_map(|prefix| {
                        [-1i16, 0, 1].into_iter().map(move |value| {
                            let mut longer = prefix.clone();
                            longer.push(value);
                            longer
                        })
                    })
                    .collect();
            }
            vectors.retain(|vector| vector.iter().any(|&value| value != 0));
            self.neighbours_along(&vectors)
        });
        neighbours_of(lists, node)
    }

    fn neighbours_along(&self, vectors: &[ExtendedPosition]) -> Vec<Vec<NodeId>> {
        self.coordinates
            .iter()
            .map(|square| {
                vectors
                    .iter()
                    .filter_map(|vector| self.offset_square(square, vector))
                    .collect()
            })
            .collect()
    }

    /// The node at `square` moved by `vector`, if that square is on the board.
    fn offset_square(&self, square: &[u8], vector: &ExtendedPosition) -> Option<NodeId> {
        if square.len() != vector.len() {
            return None;
        }
        let mut moved = Vec::with_capacity(square.len());
        for (&value, &delta) in square.iter().zip(vector) {
            let sum = i16::from(value).checked_add(delta)?;
            moved.push(u8::try_from(sum).ok()?);
        }
        self.node(&grid_name(&moved))
    }
}

fn neighbours_of(lists: &[Vec<NodeId>], node: NodeId) -> &[NodeId] {
    usize::try_from(node)
        .ok()
        .and_then(|node| lists.get(node))
        .map_or(&[], Vec::as_slice)
}

/// The name of a grid square.
fn grid_name(coordinates: &[u8]) -> Position {
    coordinates.to_vec()
}

/// Every enabled square of a grid in lexicographic order, counting like an odometer with the
/// last coordinate turning fastest.
fn grid_squares(dimensions: &[u8], disabled: &HashSet<Position>) -> Vec<Vec<u8>> {
    let mut squares = Vec::new();
    if dimensions.is_empty() || dimensions.contains(&0) {
        return squares;
    }
    let mut current = vec![0u8; dimensions.len()];
    loop {
        if !disabled.contains(&grid_name(&current)) {
            squares.push(current.clone());
        }
        let mut axis = current.len();
        loop {
            if axis == 0 {
                return squares;
            }
            axis -= 1;
            let (Some(value), Some(&size)) = (current.get_mut(axis), dimensions.get(axis)) else {
                return squares;
            };
            if *value + 1 < size {
                *value += 1;
                break;
            }
            *value = 0;
        }
    }
}
