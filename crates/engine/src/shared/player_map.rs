//! A small map keyed by player names.
//!
//! A game has a few players, so a list searched in order is faster than hashing the name each
//! time a rule is looked up for the player to move.

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerMap<V> {
    entries: Vec<(String, V)>,
}

impl<V> Default for PlayerMap<V> {
    fn default() -> Self {
        PlayerMap {
            entries: Vec::new(),
        }
    }
}

impl<V> PlayerMap<V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sets the value of `player`, returning the one it replaces.
    pub fn insert(&mut self, player: String, value: V) -> Option<V> {
        match self.entries.iter_mut().find(|(name, _)| *name == player) {
            Some((_, slot)) => Some(std::mem::replace(slot, value)),
            None => {
                self.entries.push((player, value));
                None
            }
        }
    }

    pub fn get(&self, player: &str) -> Option<&V> {
        self.entries
            .iter()
            .find(|(name, _)| name == player)
            .map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, player: &str) -> Option<&mut V> {
        self.entries
            .iter_mut()
            .find(|(name, _)| name == player)
            .map(|(_, value)| value)
    }

    pub fn contains_key(&self, player: &str) -> bool {
        self.get(player).is_some()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, value)| value)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().map(|(name, _)| name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        self.entries.iter().map(|(name, value)| (name, value))
    }
}

/// Indexing by player, for tests that know the player has a rule; it panics otherwise.
#[cfg(test)]
impl<V> std::ops::Index<&str> for PlayerMap<V> {
    type Output = V;
    fn index(&self, player: &str) -> &V {
        self.get(player).expect("a rule for this player")
    }
}

impl<V> FromIterator<(String, V)> for PlayerMap<V> {
    fn from_iter<I: IntoIterator<Item = (String, V)>>(iter: I) -> Self {
        let mut map = PlayerMap::new();
        for (player, value) in iter {
            map.insert(player, value);
        }
        map
    }
}

impl<V> IntoIterator for PlayerMap<V> {
    type Item = (String, V);
    type IntoIter = std::vec::IntoIter<(String, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a, V> IntoIterator for &'a PlayerMap<V> {
    type Item = (&'a String, &'a V);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (String, V)>,
        fn(&'a (String, V)) -> (&'a String, &'a V),
    >;
    fn into_iter(self) -> Self::IntoIter {
        fn pair<'a, V>(entry: &'a (String, V)) -> (&'a String, &'a V) {
            (&entry.0, &entry.1)
        }
        self.entries
            .iter()
            .map(pair as fn(&'a (String, V)) -> (&'a String, &'a V))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_replaces_and_lists_by_player() {
        let mut map = PlayerMap::new();
        assert_eq!(map.insert("WHITE".to_string(), 1), None);
        assert_eq!(map.insert("BLACK".to_string(), 2), None);
        assert_eq!(map.insert("WHITE".to_string(), 3), Some(1));
        assert_eq!(map.get("WHITE"), Some(&3));
        assert_eq!(map.get("BLACK"), Some(&2));
        assert!(!map.contains_key("RED"));
        assert_eq!(map.values().sum::<i32>(), 5);
        assert_eq!(map.len(), 2);
    }
}
