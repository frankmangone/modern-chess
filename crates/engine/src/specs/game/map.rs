use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

use super::board::HexLayout;
use crate::shared::Coordinates;
use crate::specs::GameSpecError;

/// The most places a map may have.
pub const MAX_PLACES: usize = 10_000;

/// The most link entries the links worked out from the places may add up to, counting the
/// quadratic ones: paths that share a point, areas that share a side and the paths along a
/// side that several areas outline. Parallel paths and duplicate areas are legal, but a few
/// thousand of them would otherwise ask for gigabytes.
pub const MAX_LINK_ENTRIES: u64 = 1_000_000;

/// The longest a place's name may be, in bytes (the game service's limit).
pub const MAX_NAME_BYTES: usize = 64;

/// How far from the origin a point's `at` may lie, in each coordinate. A larger value would
/// overflow a path's drawing position, which the view would then send as `null`.
pub const MAX_AT: f64 = 1_000_000.0;

/// The labels a map's links go by.
pub const MAP_LABELS: [&str; 6] = ["next", "ends", "paths", "corners", "areas", "sides"];

/// The labels the engine works out itself from paths and areas; a written link cannot add to them.
const DERIVED_LABELS: [&str; 5] = ["ends", "paths", "corners", "areas", "sides"];

/// Starts the message of a map refusal that travels as a JSON error while a spec is read.
const MAP_ERROR_PREFIX: &str = "invalid map: ";

/// What a place is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceKind {
    /// A dot: Catan's corners, Root's clearings.
    #[default]
    Point,
    /// A line between two points: Catan's sides, Root's paths.
    Path,
    /// A shape outlined by points: Catan's tiles, Root's forests.
    Area,
}

/// A place on a map.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceSpec {
    pub name: String,
    pub kind: PlaceKind,
    /// Where the place is drawn. Written for points; a path's is the midpoint of its ends and an
    /// area's the average of its corners.
    pub at: [f64; 2],
    /// The two points of a path.
    pub ends: Option<[String; 2]>,
    /// The points of an area's outline, in drawing order.
    pub around: Vec<String>,
    /// How many pieces the place holds; one when not written. Above one, the place is a holder:
    /// it keeps counts of pieces, never a piece of its own.
    pub capacity: Option<u32>,
    /// What a holder place holds when the game starts: piece code → count, owned by nobody.
    pub start: BTreeMap<String, u32>,
}

/// How a map was written, so that it can be written back the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapSource {
    /// `"shape": "map"`: the places and links are in the spec.
    Written,
    /// `"shape": "hex-map"`: generated from a hex layout, so only the layout is in the spec.
    HexMap {
        layout: HexLayout,
        size: u8,
        /// The tiles left out, as written (a hexagon's cut corners are not among them).
        disabled: HashSet<Coordinates>,
    },
}

/// A written link under a label of the spec's own, as names. It joins two points, one way
/// (`from` to `to`) or, with `both`, each way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelledLink {
    pub label: String,
    pub from: String,
    pub to: String,
    pub both: bool,
}

/// A checked map: places in board order and the links between them, worked out once.
#[derive(Clone, Debug, PartialEq)]
pub struct MapSpec {
    pub places: Vec<PlaceSpec>,
    /// The written links between points, as names, each both ways.
    pub lines: Vec<[String; 2]>,
    /// The written one-way links from a point to a point, as names.
    pub one_way: Vec<[String; 2]>,
    /// The written links that carry a label of the spec's own, in the order written.
    pub labelled: Vec<LabelledLink>,
    /// `links[label][i]` are the indices of the places place `i` links to under `label`, in
    /// board order without repeats. Every label of `MAP_LABELS` has an entry, and so has every
    /// label a written link names.
    pub links: BTreeMap<String, Vec<Vec<usize>>>,
    pub source: MapSource,
}

impl MapSpec {
    /// The board index of the place called `name`.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.places.iter().position(|place| place.name == name)
    }

    /// Whether a step, `AROUND` or `FOLLOW` may follow `label` on this map: one of the labels
    /// every map has, or one a written link names.
    pub fn has_label(&self, label: &str) -> bool {
        self.links.contains_key(label)
    }

    /// Checks `places` and `lines` (written links between points) and works out every link.
    /// The error names the place at fault.
    pub fn compile(
        mut places: Vec<PlaceSpec>,
        lines: Vec<[String; 2]>,
        one_way: Vec<[String; 2]>,
        labelled: Vec<LabelledLink>,
        source: MapSource,
    ) -> Result<MapSpec, String> {
        if places.is_empty() {
            return Err("the map has no places".into());
        }
        if places.len() > MAX_PLACES {
            return Err(format!("the map has more than {MAX_PLACES} places"));
        }
        let mut index: HashMap<&str, usize> = HashMap::new();
        for (at, place) in places.iter().enumerate() {
            let name = place.name.as_str();
            if name.is_empty() {
                return Err(format!("place {at} has an empty name"));
            }
            if name.len() > MAX_NAME_BYTES {
                return Err(format!(
                    "the place name `{name}` is {} bytes; a name is 1 to {MAX_NAME_BYTES} bytes",
                    name.len()
                ));
            }
            if index.insert(name, at).is_some() {
                return Err(format!("the place name `{name}` is used twice"));
            }
        }
        let point = |name: &str, whose: &str| -> Result<usize, String> {
            let found = index
                .get(name)
                .copied()
                .ok_or_else(|| format!("{whose} names `{name}`, which is not a place"))?;
            match places.get(found) {
                Some(place) if place.kind == PlaceKind::Point => Ok(found),
                _ => Err(format!("{whose} names `{name}`, which is not a point")),
            }
        };

        let count = places.len();
        let mut ends: Vec<Option<[usize; 2]>> = vec![None; count];
        let mut outlines: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (at, place) in places.iter().enumerate() {
            let whose = format!("`{}`", place.name);
            match place.kind {
                PlaceKind::Point => {
                    if place.ends.is_some() || !place.around.is_empty() {
                        return Err(format!(
                            "the point {whose} takes `at`, not `ends` or `around`"
                        ));
                    }
                }
                PlaceKind::Path => {
                    let Some([first, second]) = &place.ends else {
                        return Err(format!("the path {whose} needs two `ends`"));
                    };
                    if !place.around.is_empty() {
                        return Err(format!("the path {whose} takes `ends`, not `around`"));
                    }
                    let (first, second) = (point(first, &whose)?, point(second, &whose)?);
                    if first == second {
                        return Err(format!("the path {whose} has the same point at both ends"));
                    }
                    if let Some(slot) = ends.get_mut(at) {
                        *slot = Some([first.min(second), first.max(second)]);
                    }
                }
                PlaceKind::Area => {
                    if place.ends.is_some() {
                        return Err(format!("the area {whose} takes `around`, not `ends`"));
                    }
                    if place.around.len() < 3 {
                        return Err(format!(
                            "the area {whose} needs at least three points `around` it"
                        ));
                    }
                    let mut corners = Vec::new();
                    for name in &place.around {
                        let corner = point(name, &whose)?;
                        if corners.contains(&corner) {
                            return Err(format!("the area {whose} goes around `{name}` twice"));
                        }
                        corners.push(corner);
                    }
                    if let Some(slot) = outlines.get_mut(at) {
                        *slot = corners;
                    }
                }
            }
        }

        let mut written: Vec<(usize, usize)> = Vec::new();
        for [first, second] in &lines {
            let whose = format!("the link `{first}`-`{second}`");
            let (first, second) = (point(first, &whose)?, point(second, &whose)?);
            if first == second {
                return Err(format!("{whose} joins a point to itself"));
            }
            written.push((first, second));
        }
        let mut directed: Vec<(usize, usize)> = Vec::new();
        for [first, second] in &one_way {
            let whose = format!("the one-way link `{first}`->`{second}`");
            let (first, second) = (point(first, &whose)?, point(second, &whose)?);
            if first == second {
                return Err(format!("{whose} joins a point to itself"));
            }
            directed.push((first, second));
        }
        let mut named: Vec<(&str, usize, usize, bool)> = Vec::new();
        for link in &labelled {
            let label = link.label.as_str();
            if label.is_empty() {
                return Err("a link has an empty label".into());
            }
            if label.len() > MAX_NAME_BYTES {
                return Err(format!(
                    "the link label `{label}` is {} bytes; a label is 1 to {MAX_NAME_BYTES} bytes",
                    label.len()
                ));
            }
            if DERIVED_LABELS.contains(&label) {
                return Err(format!(
                    "the link label `{label}` is worked out from the paths and areas; a written \
                     link cannot add to it"
                ));
            }
            let whose = format!("the link `{}`-`{}` labelled `{label}`", link.from, link.to);
            let (first, second) = (point(&link.from, &whose)?, point(&link.to, &whose)?);
            if first == second {
                return Err(format!("{whose} joins a point to itself"));
            }
            named.push((label, first, second, link.both));
        }

        let mut paths_by_ends: HashMap<[usize; 2], Vec<usize>> = HashMap::new();
        let mut paths_at: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (path, pair) in ends.iter().enumerate() {
            let Some([first, second]) = *pair else {
                continue;
            };
            paths_by_ends.entry([first, second]).or_default().push(path);
            for end in [first, second] {
                if let Some(at_end) = paths_at.get_mut(end) {
                    at_end.push(path);
                }
            }
        }
        let mut areas_by_side: HashMap<[usize; 2], Vec<usize>> = HashMap::new();
        for (area, corners) in outlines.iter().enumerate() {
            for (position, &corner) in corners.iter().enumerate() {
                let following = corners.get((position + 1) % corners.len()).copied();
                let Some(following) = following else {
                    continue;
                };
                let side = [corner.min(following), corner.max(following)];
                areas_by_side.entry(side).or_default().push(area);
            }
        }
        check_link_budget(&places, &paths_at, &paths_by_ends, &areas_by_side)?;

        let mut links = Links::new(count, labelled.iter().map(|link| link.label.as_str()));
        for (path, pair) in ends.iter().enumerate() {
            let Some([first, second]) = *pair else {
                continue;
            };
            for (end, other) in [(first, second), (second, first)] {
                links.add("next", end, other);
                links.add("paths", end, path);
            }
            links.add("ends", path, first);
            links.add("ends", path, second);
        }
        for &(first, second) in &written {
            links.add("next", first, second);
            links.add("next", second, first);
        }
        for &(first, second) in &directed {
            links.add("next", first, second);
        }
        for &(label, first, second, both) in &named {
            links.add(label, first, second);
            if both {
                links.add(label, second, first);
            }
        }
        for here in &paths_at {
            for &path in here {
                for &other in here {
                    if path != other {
                        links.add("next", path, other);
                    }
                }
            }
        }
        for (area, corners) in outlines.iter().enumerate() {
            for (position, &corner) in corners.iter().enumerate() {
                links.add("corners", area, corner);
                links.add("areas", corner, area);
                let following = corners.get((position + 1) % corners.len()).copied();
                let Some(following) = following else {
                    continue;
                };
                let side = [corner.min(following), corner.max(following)];
                for &path in paths_by_ends.get(&side).into_iter().flatten() {
                    links.add("sides", area, path);
                    links.add("areas", path, area);
                }
            }
        }
        for sharing in areas_by_side.values() {
            for &area in sharing {
                for &other in sharing {
                    if area != other {
                        links.add("next", area, other);
                    }
                }
            }
        }

        let mut tidy = links.finish();
        for lists in tidy.values_mut() {
            for list in lists.iter_mut() {
                list.sort_unstable();
                list.dedup();
            }
        }

        let mut at_of: Vec<[f64; 2]> = places.iter().map(|place| place.at).collect();
        for (place, pair) in ends.iter().enumerate() {
            let Some([first, second]) = *pair else {
                continue;
            };
            let ends_at: Vec<[f64; 2]> = [first, second]
                .iter()
                .filter_map(|&end| at_of.get(end).copied())
                .collect();
            if let Some(slot) = at_of.get_mut(place) {
                *slot = average(&ends_at);
            }
        }
        for (place, corners) in outlines.iter().enumerate() {
            if corners.is_empty() {
                continue;
            }
            let at_corners: Vec<[f64; 2]> = corners
                .iter()
                .filter_map(|&corner| at_of.get(corner).copied())
                .collect();
            if let Some(slot) = at_of.get_mut(place) {
                *slot = average(&at_corners);
            }
        }
        for (place, at) in places.iter_mut().zip(at_of) {
            place.at = at;
        }

        Ok(MapSpec {
            places,
            lines,
            one_way,
            labelled,
            links: tidy,
            source,
        })
    }
}

/// Refuses a map whose links would add up to more than `MAX_LINK_ENTRIES`, naming the point or
/// side that asks for the most. Counts before any list is built.
fn check_link_budget(
    places: &[PlaceSpec],
    paths_at: &[Vec<usize>],
    paths_by_ends: &HashMap<[usize; 2], Vec<usize>>,
    areas_by_side: &HashMap<[usize; 2], Vec<usize>>,
) -> Result<(), String> {
    let name = |at: usize| places.get(at).map_or("", |place| place.name.as_str());
    let mut total: u64 = 0;
    let mut heaviest: (u64, String) = (0, String::new());
    let mut count = |entries: u64, what: &dyn Fn() -> String| {
        total = total.saturating_add(entries);
        if entries > heaviest.0 {
            heaviest = (entries, what());
        }
    };
    for (point, here) in paths_at.iter().enumerate() {
        let paths = here.len() as u64;
        count(paths * paths.saturating_sub(1), &|| {
            format!("the {paths} paths at the point `{}`", name(point))
        });
    }
    for (side, areas) in areas_by_side {
        let areas_count = areas.len() as u64;
        let paths = paths_by_ends.get(side).map_or(0, Vec::len) as u64;
        let entries = areas_count * areas_count.saturating_sub(1) + 2 * areas_count * paths;
        count(entries, &|| {
            format!(
                "the {areas_count} areas along the side `{}`-`{}`",
                name(side[0]),
                name(side[1])
            )
        });
    }
    if total > MAX_LINK_ENTRIES {
        return Err(format!(
            "the links of this map would have {total} entries, more than {MAX_LINK_ENTRIES}; \
             the most come from {}",
            heaviest.1
        ));
    }
    Ok(())
}

/// The point halfway along or at the centre of `points`.
fn average(points: &[[f64; 2]]) -> [f64; 2] {
    let count = points.len() as f64;
    let (x, y) = points
        .iter()
        .fold((0.0, 0.0), |(x, y), at| (x + at[0], y + at[1]));
    [x / count, y / count]
}

/// The link lists as they are built, one list per place for each label.
struct Links(BTreeMap<String, Vec<Vec<usize>>>);

impl Links {
    fn new<'a>(places: usize, written: impl Iterator<Item = &'a str>) -> Links {
        Links(
            MAP_LABELS
                .iter()
                .copied()
                .chain(written)
                .map(|label| (label.to_string(), vec![Vec::new(); places]))
                .collect(),
        )
    }

    fn add(&mut self, label: &str, from: usize, to: usize) {
        if let Some(list) = self.0.get_mut(label).and_then(|lists| lists.get_mut(from)) {
            list.push(to);
        }
    }

    fn finish(self) -> BTreeMap<String, Vec<Vec<usize>>> {
        self.0
    }
}

/// A link as a spec writes it: `["a", "b"]` joins two points both ways, `{ "from": "a", "to": "b" }`
/// goes one way.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(super) enum RawLink {
    Both([String; 2]),
    OneWay(OneWayLink),
    Between(BetweenLink),
}

/// A link that goes one way. It refuses a key it does not know, so a misspelled `from` or an
/// added label is not passed over in silence.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OneWayLink {
    pub from: String,
    pub to: String,
    /// The label it goes under; `next` when not written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A link that goes both ways under a label, written `{ "between": ["a", "b"], "label": "L" }`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BetweenLink {
    pub between: [String; 2],
    pub label: String,
}

/// A place as a spec writes it.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPlace {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<PlaceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends: Option<[String; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub around: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub start: BTreeMap<String, u32>,
}

impl RawPlace {
    /// The place as written, before the drawing positions of paths and areas are worked out.
    fn into_place(self) -> Result<PlaceSpec, String> {
        let kind = self.kind.unwrap_or_default();
        if kind != PlaceKind::Point && self.at.is_some() {
            return Err(format!(
                "`{}` is not a point, so it takes no `at`: it is drawn from its points",
                self.name
            ));
        }
        let at = match (kind, self.at) {
            (PlaceKind::Point, None) => {
                return Err(format!("the point `{}` needs an `at`", self.name));
            }
            (_, at) => at.unwrap_or([0.0, 0.0]),
        };
        if at
            .iter()
            .any(|value| !value.is_finite() || value.abs() > MAX_AT)
        {
            return Err(format!(
                "the `at` of `{}` must be within {MAX_AT} of the origin in each coordinate",
                self.name
            ));
        }
        Ok(PlaceSpec {
            name: self.name,
            kind,
            at,
            ends: self.ends,
            around: self.around.unwrap_or_default(),
            capacity: self.capacity,
            start: self.start,
        })
    }
}

impl From<&PlaceSpec> for RawPlace {
    fn from(place: &PlaceSpec) -> RawPlace {
        RawPlace {
            name: place.name.clone(),
            kind: (place.kind != PlaceKind::Point).then_some(place.kind),
            at: (place.kind == PlaceKind::Point).then_some(place.at),
            ends: place.ends.clone(),
            around: (place.kind == PlaceKind::Area).then(|| place.around.clone()),
            capacity: place.capacity,
            start: place.start.clone(),
        }
    }
}

/// Compiles a written map, giving a refusal the prefix that `map_refusal` looks for.
pub(super) fn compile_written(
    places: Vec<RawPlace>,
    links: Vec<RawLink>,
) -> Result<MapSpec, String> {
    let mut lines = Vec::new();
    let mut one_way = Vec::new();
    let mut labelled = Vec::new();
    let mut put = |label: Option<String>, from: String, to: String, both: bool| match label
        .filter(|label| label != "next")
    {
        Some(label) => labelled.push(LabelledLink {
            label,
            from,
            to,
            both,
        }),
        None if both => lines.push([from, to]),
        None => one_way.push([from, to]),
    };
    for link in links {
        match link {
            RawLink::Both([from, to]) => put(None, from, to, true),
            RawLink::OneWay(OneWayLink { from, to, label }) => put(label, from, to, false),
            RawLink::Between(BetweenLink {
                between: [from, to],
                label,
            }) => put(Some(label), from, to, true),
        }
    }
    let places = places
        .into_iter()
        .map(RawPlace::into_place)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|why| format!("{MAP_ERROR_PREFIX}{why}"))?;
    MapSpec::compile(places, lines, one_way, labelled, MapSource::Written)
        .map_err(|why| format!("{MAP_ERROR_PREFIX}{why}"))
}

/// The refusal a spec's map earned, when `error` (from reading a spec) carries one.
pub(crate) fn map_refusal(error: &serde_json::Error) -> Option<GameSpecError> {
    let message = error.to_string();
    let rest = message.strip_prefix(MAP_ERROR_PREFIX)?;
    let why = rest.rsplit_once(" at line ").map_or(rest, |(why, _)| why);
    Some(GameSpecError::InvalidMap(why.to_string()))
}

/// The prefix for a refusal about a board that is not a map's but is written with a map's
/// fields, or the other way round.
pub(super) fn map_refused(why: impl std::fmt::Display) -> String {
    format!("{MAP_ERROR_PREFIX}{why}")
}
