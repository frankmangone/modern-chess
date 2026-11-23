//! The map form of the board graph: a map's links, stored as one flat table per label.

use std::sync::Arc;

use super::graph::{LabelId, Labels, NodeId};
use crate::specs::MapSpec;

/// The links of one label: the nodes place `i` links to are
/// `targets[offsets[i]..offsets[i + 1]]`, in board order.
#[derive(Clone, Debug, Default)]
struct Table {
    offsets: Vec<u32>,
    targets: Vec<NodeId>,
}

impl Table {
    /// Flattens one list of linked place indices per place.
    fn new(lists: &[Vec<usize>]) -> Table {
        let mut offsets = Vec::with_capacity(lists.len() + 1);
        let mut targets = Vec::with_capacity(lists.iter().map(Vec::len).sum());
        offsets.push(0);
        for list in lists {
            targets.extend(list.iter().filter_map(|&to| NodeId::try_from(to).ok()));
            offsets.push(u32::try_from(targets.len()).unwrap_or(u32::MAX));
        }
        Table { offsets, targets }
    }

    fn linked(&self, node: NodeId) -> &[NodeId] {
        let Ok(node) = usize::try_from(node) else {
            return &[];
        };
        let (Some(&start), Some(&end)) = (self.offsets.get(node), self.offsets.get(node + 1))
        else {
            return &[];
        };
        let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
            return &[];
        };
        self.targets.get(start..end).unwrap_or(&[])
    }
}

/// A map's links: a table for each of the map labels, and which of them each of the game's
/// labels reads.
#[derive(Clone, Debug)]
pub struct MapLinks {
    spec: Arc<MapSpec>,
    /// One table per label of the map, in the order of its `links`.
    tables: Vec<Table>,
    /// For each `LabelId`, the index of its table; `None` for a label maps do not have.
    by_label: Vec<Option<usize>>,
    /// The index of the `next` table, which is the map's `orthogonal` and `adjacent`.
    next: Option<usize>,
}

impl MapLinks {
    pub fn new(spec: &Arc<MapSpec>, labels: &Labels) -> MapLinks {
        let tables: Vec<Table> = spec.links.values().map(|lists| Table::new(lists)).collect();
        let table_of = |name: &str| spec.links.keys().position(|label| label == name);
        let mut by_label = vec![None; labels.vectors().len()];
        for (name, label) in labels.names() {
            if let Some(slot) = usize::try_from(label)
                .ok()
                .and_then(|label| by_label.get_mut(label))
            {
                *slot = table_of(name);
            }
        }
        MapLinks {
            spec: Arc::clone(spec),
            tables,
            by_label,
            next: table_of("next"),
        }
    }

    pub fn spec(&self) -> &MapSpec {
        &self.spec
    }

    /// The nodes `label` links `node` to.
    pub fn linked(&self, node: NodeId, label: LabelId) -> &[NodeId] {
        let table = usize::try_from(label)
            .ok()
            .and_then(|label| self.by_label.get(label).copied().flatten());
        self.table(table, node)
    }

    /// The nodes `next` links `node` to.
    pub fn next(&self, node: NodeId) -> &[NodeId] {
        self.table(self.next, node)
    }

    fn table(&self, table: Option<usize>, node: NodeId) -> &[NodeId] {
        table
            .and_then(|table| self.tables.get(table))
            .map_or(&[], |table| table.linked(node))
    }
}
