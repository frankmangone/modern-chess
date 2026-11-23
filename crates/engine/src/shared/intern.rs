//! Names as small ids.
//!
//! A name the engine compares and hashes all the time (a place, a piece code, a player, a flag)
//! is interned once: the text goes into a global table and the name becomes a pointer to its
//! entry. Two names are equal when their pointers are, and the text is read from the entry. The
//! table only grows and its texts are never freed, so a text can be lent for the life of the
//! program; the names a game uses are few (its board's places, codes, players and flags).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One interned text. Every distinct text has exactly one entry, so entries are equal when
/// their addresses are.
pub(crate) struct Entry {
    text: Box<str>,
}

impl Entry {
    pub(crate) fn as_str(&'static self) -> &'static str {
        &self.text
    }
}

#[derive(Default)]
struct Table {
    entries: HashMap<&'static str, &'static Entry>,
}

fn table() -> &'static RwLock<Table> {
    static TABLE: OnceLock<RwLock<Table>> = OnceLock::new();
    TABLE.get_or_init(|| RwLock::new(Table::default()))
}

/// The entry of `text` when it is already interned; the table is not changed.
pub(crate) fn find(text: &str) -> Option<&'static Entry> {
    let table = table()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    table.entries.get(text).copied()
}

/// The entry of `text`, added to the table when it is new.
pub(crate) fn intern(text: &str) -> &'static Entry {
    if let Ok(table) = table().read() {
        if let Some(&entry) = table.entries.get(text) {
            return entry;
        }
    }
    let mut table = table()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(&entry) = table.entries.get(text) {
        return entry;
    }
    let entry: &'static Entry = Box::leak(Box::new(Entry { text: text.into() }));
    table.entries.insert(entry.as_str(), entry);
    entry
}

/// The address of an entry, which names it: equal texts share one entry.
pub(crate) fn address(entry: &'static Entry) -> usize {
    entry as *const Entry as usize
}

/// A name that stands for a piece code, a player or a flag: a pointer to its interned text that
/// compares and hashes as itself, and orders, prints and saves as the text it stands for.
#[derive(Clone, Copy)]
pub struct Symbol(&'static Entry);

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Symbol {}

impl std::hash::Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        address(self.0).hash(state)
    }
}

impl Symbol {
    pub fn new(text: &str) -> Self {
        Symbol(intern(text))
    }

    pub fn as_str(&self) -> &'static str {
        self.0.as_str()
    }
}

impl Default for Symbol {
    fn default() -> Self {
        Symbol::new("")
    }
}

impl Deref for Symbol {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for Symbol {
    fn from(text: &str) -> Self {
        Symbol::new(text)
    }
}

impl From<&String> for Symbol {
    fn from(text: &String) -> Self {
        Symbol::new(text)
    }
}

impl From<String> for Symbol {
    fn from(text: String) -> Self {
        Symbol::new(&text)
    }
}

impl From<Symbol> for String {
    fn from(symbol: Symbol) -> String {
        symbol.as_str().to_string()
    }
}

impl PartialEq<str> for Symbol {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Symbol {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Symbol {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Symbol> for str {
    fn eq(&self, other: &Symbol) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<Symbol> for &str {
    fn eq(&self, other: &Symbol) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<Symbol> for String {
    fn eq(&self, other: &Symbol) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            return Ordering::Equal;
        }
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Serialize for Symbol {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = std::borrow::Cow::<str>::deserialize(deserializer)?;
        Ok(Symbol::new(&text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_text_same_id() {
        assert_eq!(Symbol::new("KING"), Symbol::new("KING"));
        assert_ne!(Symbol::new("KING"), Symbol::new("QUEEN"));
    }

    #[test]
    fn symbol_equals_its_text() {
        let king = Symbol::new("KING");
        assert!(king == *"KING");
        assert!(king == "KING");
        assert!(king == "KING".to_string());
        assert!("KING" == king);
        assert_eq!(king.as_str(), "KING");
        assert_eq!(king.len(), 4);
    }

    #[test]
    fn symbol_orders_by_text() {
        let mut names = vec![
            Symbol::new("ROOK"),
            Symbol::new("BISHOP"),
            Symbol::new("KING"),
        ];
        names.sort();
        assert_eq!(names, ["BISHOP", "KING", "ROOK"].map(Symbol::new));
    }

    #[test]
    fn symbol_saves_as_its_text() {
        let json = serde_json::to_string(&Symbol::new("WHITE")).unwrap();
        assert_eq!(json, "\"WHITE\"");
        let back: Symbol = serde_json::from_str(&json).unwrap();
        assert_eq!(back, "WHITE");
    }

    #[test]
    fn the_table_is_shared_between_threads() {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    (0..200)
                        .map(|n| Symbol::new(&format!("T{n}")))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let all: Vec<Vec<Symbol>> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert!(all.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(all[0][7], "T7");
    }
}
