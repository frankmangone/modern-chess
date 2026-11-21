use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::shared::{apply_direction, Coordinates, Direction, ExtendedPosition, Position};
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

/// The board as the engine walks it: named nodes, edges labelled by the vectors the rules use,
/// and the `orthogonal` and `adjacent` neighbourhoods.
///
/// On a grid an edge is not stored: following label `v` from a square looks up the square `v`
/// away in a dense index of the grid. Building a game therefore costs one entry per square
/// whatever its rules, and a spec with thousands of moves or eight dimensions stays cheap.
pub struct Graph {
    names: Vec<Position>,
    index: HashMap<Position, NodeId>,
    coordinates: Vec<Vec<u8>>,
    /// The size of each axis of the grid.
    dimensions: Vec<u8>,
    /// Every square of the grid, last coordinate turning fastest, holding its node (or
    /// `NO_NODE` for a disabled square).
    slots: Vec<NodeId>,
    /// The vector of each label.
    labels: Vec<ExtendedPosition>,
    orthogonal: Vec<ExtendedPosition>,
    adjacent: Vec<ExtendedPosition>,
}

impl Graph {
    /// Compiles a grid into a graph. Squares become nodes named by their coordinates, numbered
    /// in lexicographic order (first coordinate outermost) with disabled squares left out. Each
    /// label is an edge from every square to the square its vector reaches.
    pub fn grid(spec: &BoardSpec, labels: &Labels) -> Graph {
        let dimensions = spec.dimensions.clone();
        let coordinates = grid_squares(&dimensions, &spec.disabled_positions);
        let names: Vec<Position> = coordinates.iter().map(|c| grid_name(c)).collect();
        let index = names
            .iter()
            .enumerate()
            .filter_map(|(node, name)| Some((name.clone(), NodeId::try_from(node).ok()?)))
            .collect();
        let size = dimensions
            .iter()
            .try_fold(1usize, |total, &axis| total.checked_mul(usize::from(axis)))
            .unwrap_or(0);
        let mut graph = Graph {
            names,
            index,
            coordinates,
            orthogonal: orthogonal_vectors(dimensions.len()),
            adjacent: adjacent_vectors(dimensions.len()),
            dimensions,
            slots: vec![NO_NODE; size],
            labels: labels.vectors().to_vec(),
        };
        for (node, square) in graph.coordinates.iter().enumerate() {
            let (Some(slot), Ok(node)) = (graph.slot(square, None), NodeId::try_from(node)) else {
                continue;
            };
            if let Some(entry) = graph.slots.get_mut(slot) {
                *entry = node;
            }
        }
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
        let vector = self.labels.get(usize::try_from(label).ok()?)?;
        self.offset_node(node, vector)
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
    pub fn orthogonal(&self, node: NodeId) -> Vec<NodeId> {
        self.along(node, &self.orthogonal)
    }

    /// The squares within one step along every axis.
    pub fn adjacent(&self, node: NodeId) -> Vec<NodeId> {
        self.along(node, &self.adjacent)
    }

    fn along(&self, node: NodeId, vectors: &[ExtendedPosition]) -> Vec<NodeId> {
        vectors
            .iter()
            .filter_map(|vector| self.offset_node(node, vector))
            .collect()
    }

    /// The node `vector` away from `node`, if that square is on the board.
    fn offset_node(&self, node: NodeId, vector: &ExtendedPosition) -> Option<NodeId> {
        let square = self.coordinates(node)?;
        let slot = self.slot(square, Some(vector))?;
        let next = *self.slots.get(slot)?;
        (next != NO_NODE).then_some(next)
    }

    /// Where `square` moved by `vector` sits in `slots`, if it is inside the grid.
    fn slot(&self, square: &[u8], vector: Option<&ExtendedPosition>) -> Option<usize> {
        if square.len() != self.dimensions.len()
            || vector.is_some_and(|vector| vector.len() != square.len())
        {
            return None;
        }
        let mut slot = 0usize;
        for (axis, (&value, &size)) in square.iter().zip(&self.dimensions).enumerate() {
            let delta = vector
                .and_then(|vector| vector.get(axis))
                .copied()
                .unwrap_or(0);
            let moved = i16::from(value).checked_add(delta)?;
            let moved = usize::try_from(moved).ok()?;
            if moved >= usize::from(size) {
                return None;
            }
            slot = slot.checked_mul(usize::from(size))?.checked_add(moved)?;
        }
        Some(slot)
    }
}

/// A summary: printing a game should not list every square of its board.
impl fmt::Debug for Graph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Graph")
            .field("dimensions", &self.dimensions)
            .field("nodes", &self.names.len())
            .field("labels", &self.labels.len())
            .finish_non_exhaustive()
    }
}

/// One step back and one forward along each axis, axis by axis.
fn orthogonal_vectors(axes: usize) -> Vec<ExtendedPosition> {
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
    vectors
}

/// Every vector of -1, 0 and 1 along each axis except the zero vector.
fn adjacent_vectors(axes: usize) -> Vec<ExtendedPosition> {
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
    vectors
}

/// The name of a grid square.
fn grid_name(coordinates: &[u8]) -> Position {
    Position::from_coordinates(coordinates)
}

/// Every enabled square of a grid in lexicographic order, counting like an odometer with the
/// last coordinate turning fastest.
fn grid_squares(dimensions: &[u8], disabled: &HashSet<Coordinates>) -> Vec<Vec<u8>> {
    let mut squares = Vec::new();
    if dimensions.is_empty() || dimensions.contains(&0) {
        return squares;
    }
    let mut current = vec![0u8; dimensions.len()];
    loop {
        if !disabled.contains(&current) {
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
