use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;

use chrono::prelude::*;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::cli::error::{Error, VResult};

/// The contents of this are an implementation detail
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ItemId(Uuid);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Book {
    events: Vec<EventFrame>,
    created: DateTime<Utc>,
}
impl Default for Book {
    fn default() -> Self {
        Self::new()
    }
}
impl Book {
    pub fn new() -> Book {
        Book {
            events: Vec::new(),
            created: Utc::now(),
        }
    }

    fn next_id(&mut self) -> ItemId {
        ItemId(Uuid::new_v4())
    }

    pub fn creation_time(&self) -> DateTime<Utc> {
        self.created
    }

    /// Create and update with data
    pub fn add(&mut self, item: Item) -> VResult<ItemId> {
        self.verify_not_exists(&item.name)?;
        let item_id = self.next_id();
        let time = Utc::now();
        self.events.push(EventFrame {
            time,
            event: Event::Create(item_id),
        });
        self.events.push(EventFrame {
            time,
            event: Event::Update(item_id, item),
        });
        Ok(item_id)
    }

    /// Updates item using new value
    pub fn update(&mut self, item_id: ItemId, item: Item) {
        if !self.item_ids().contains(&item_id) {
            panic!("Cannot update nonexistent ItemId");
        }
        self.events.push(EventFrame {
            time: Utc::now(),
            event: Event::Update(item_id, item),
        });
    }

    /// Updates item by mapping the old value
    #[must_use]
    fn modify<F, R>(&mut self, item_id: ItemId, f: F) -> Option<R>
    where F: FnOnce(&mut Item) -> R {
        let mut item = self.read_item(item_id)?;
        let r = f(&mut item);
        self.update(item_id, item);
        Some(r)
    }

    /// Updates item by mapping the old value
    #[must_use]
    pub fn modify_by_name<F, R>(&mut self, name: &str, f: F) -> VResult<R>
    where F: FnOnce(&mut Item) -> R {
        Ok(self
            .modify(
                self.find_id_by_name(name)
                    .ok_or_else(|| Error::NoSuchItem(name.to_owned()))?,
                f,
            )
            .unwrap())
    }

    pub fn remove(&mut self, name: &str) -> VResult<()> {
        let id = self.get_id_by_name(name)?;
        self.events.push(EventFrame {
            time: Utc::now(),
            event: Event::Remove(id),
        });
        Ok(())
    }

    /// All ItemId:s, including removed ones
    fn all_ids(&self) -> HashSet<ItemId> {
        self.events.iter().filter_map(EventFrame::creates_id).collect()
    }

    /// Removed ItemId:s
    fn removed_ids(&self) -> HashSet<ItemId> {
        self.events.iter().filter_map(EventFrame::removes_id).collect()
    }

    fn item_ids(&self) -> HashSet<ItemId> {
        self.all_ids().difference(&self.removed_ids()).copied().collect()
    }

    #[must_use]
    fn read_item(&self, id: ItemId) -> Option<Item> {
        for ef in self.events.iter().rev() {
            match ef.clone().event {
                Event::Update(e_id, event) if e_id == id => {
                    return Some(event);
                },
                Event::Create(e_id) if e_id == id => {
                    unreachable!("Item created but not initialized");
                },
                _ => {},
            }
        }
        None
    }

    #[must_use]
    fn read_item_metadata(&self, id: ItemId) -> Option<ItemMetadata> {
        let mut created: Option<DateTime<Utc>> = None;
        let mut changed: Option<DateTime<Utc>> = None;

        for ef in self.events.iter() {
            match ef.clone().event {
                Event::Create(e_id) if e_id == id => {
                    created = Some(ef.time);
                },
                Event::Update(e_id, _) if e_id == id => {
                    changed = Some(ef.time);
                },
                _ => {},
            }
        }

        Some(ItemMetadata {
            created: created?,
            changed: changed?,
        })
    }

    pub fn items(&self) -> Vec<Item> {
        self.item_ids()
            .into_iter()
            .map(|id| self.read_item(id).unwrap())
            .collect()
    }

    fn id_items(&self) -> Vec<(ItemId, Item)> {
        self.item_ids()
            .into_iter()
            .map(|id| (id, self.read_item(id).unwrap()))
            .collect()
    }

    /// Items and associated metadata
    pub fn items_metadata(&self) -> Vec<(Item, ItemMetadata)> {
        self.item_ids()
            .into_iter()
            .map(|id| (self.read_item(id).unwrap(), self.read_item_metadata(id).unwrap()))
            .collect()
    }

    /// Items and associated metadata
    pub fn get_item_and_metadata(&self, name: &str) -> VResult<(Item, ItemMetadata)> {
        let id = self.get_id_by_name(name)?;
        Ok((self.read_item(id).unwrap(), self.read_item_metadata(id).unwrap()))
    }

    fn find_id<F>(&self, f: F) -> Option<ItemId>
    where F: Fn(&Item) -> bool {
        self.id_items()
            .iter()
            .find(|(_, item)| f(item))
            .map(|(id, _)| *id)
    }

    fn find_id_by_name(&self, name: &str) -> Option<ItemId> {
        self.find_id(|item| item.name == name)
    }

    pub fn has_item(&self, name: &str) -> bool {
        self.find_id_by_name(name).is_some()
    }

    fn get_id_by_name(&self, name: &str) -> VResult<ItemId> {
        self.find_id_by_name(name)
            .ok_or_else(|| Error::NoSuchItem(name.to_owned()))
    }

    pub fn get_item_by_name(&self, name: &str) -> VResult<Item> {
        Ok(self.read_item(self.get_id_by_name(name)?).unwrap())
    }

    pub fn verify_exists(&self, name: &str) -> VResult<()> {
        if self.has_item(name) {
            Ok(())
        } else {
            Err(Error::NoSuchItem(name.to_owned()))
        }
    }

    pub fn verify_not_exists(&self, name: &str) -> VResult<()> {
        if self.has_item(name) {
            Err(Error::ItemAlreadyExists(name.to_owned()))
        } else {
            Ok(())
        }
    }

    pub fn item_count(&self) -> usize {
        self.items().len()
    }

    pub fn item_names(&self) -> Vec<String> {
        self.items().iter().map(|item| item.name.clone()).collect()
    }

    /// Index of the the last common event
    /// Must be only used with two books with common history
    /// Returns None in case the event lists are equal
    fn differ_index(&self, other: &Self) -> Option<usize> {
        debug_assert_eq!(self.created, other.created);
        for (i, (s, o)) in self.events.iter().zip(other.events.iter()).enumerate() {
            if s.time != o.time {
                return Some(i);
            }
        }
        if self.events.len() == other.events.len() {
            None
        } else {
            Some(self.events.len().min(other.events.len()))
        }
    }

    /// Check if two books have same origin, i.e. creation time, and can be merged together
    #[must_use]
    pub fn has_same_origin(&self, other: &Self) -> bool {
        self.created != other.created
    }

    /// Merge two versions of one password book together
    #[must_use]
    pub fn merge_versions(mut self, other: &Self) -> Result<Self, VersionMergeError> {
        if self.has_same_origin(other) {
            Err(VersionMergeError::DifferentOrigins)
        } else if let Some(di) = self.differ_index(&other) {
            println!("{:?}", di);
            // Remove new events from self, making it the common prefix
            let mut tail = self.events.split_off(di);
            // Sort only new events, and append them
            tail.extend(other.events.iter().skip(di).cloned().collect::<Vec<_>>());
            tail.sort();
            for t in &tail {
                println!("{:?}", t);
            }
            self.events.extend(tail);
            self.clean();
            Ok(self)
        } else {
            // The books are equal
            Ok(self)
        }
    }

    /// Remove unnecessary events, such as multiple removes
    fn clean(&mut self) {
        // Multiple removes
        let mut removed: HashSet<ItemId> = HashSet::new();
        for ef in self.events.clone() {
            if let Some(id) = ef.removes_id() {
                if removed.contains(&id) {
                    self.events.remove_item(&ef);
                } else {
                    removed.insert(id);
                }
            }
        }
    }
}

/// Error when merging two versions of a password book together
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum VersionMergeError {
    DifferentOrigins,
}

/// An event and it's context in the book
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct EventFrame {
    time: DateTime<Utc>,
    event: Event,
}
impl EventFrame {
    fn creates_id(&self) -> Option<ItemId> {
        self.event.creates_id()
    }
    fn removes_id(&self) -> Option<ItemId> {
        self.event.removes_id()
    }
}
impl PartialOrd for EventFrame {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}
impl Ord for EventFrame {
    fn cmp(&self, other: &EventFrame) -> Ordering {
        self.time.cmp(&other.time)
    }
}

/// Actual event that occurred
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
enum Event {
    Create(ItemId),
    Update(ItemId, Item),
    Remove(ItemId),
}
impl Event {
    fn creates_id(&self) -> Option<ItemId> {
        match self {
            Event::Create(id) => Some(*id),
            _ => None,
        }
    }
    fn removes_id(&self) -> Option<ItemId> {
        match self {
            Event::Remove(id) => Some(*id),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ItemMetadata {
    pub created: DateTime<Utc>,
    pub changed: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Item {
    /// Although all fields have ids, names must still be unique
    /// Name schema, by-convention:
    ///     * Folders can be emulated with "path/to/filename"
    ///     * Folder "vpass/" contains internal vpass items
    pub name: String,
    /// Password itself, if set
    pub password: Option<Password>,
    /// One word tags
    pub tags: HashSet<String>,
    /// Free-form text notes
    pub notes: Vec<String>,
}
impl Item {
    pub fn new(name: &str) -> Item {
        Item {
            name: name.to_owned(),
            password: None,
            tags: HashSet::new(),
            notes: Vec::new(),
        }
    }
}

/// Plaintext password.
/// A separate struct is used to hide plaintext password from debug output.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Password(String);
impl Password {
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}
impl Password {
    /// Read plaintext password
    pub fn plaintext(&self) -> String {
        self.0.clone()
    }
}
impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Password(****)")
    }
}

#[cfg(test)]
mod tests {
    use super::{Book, Item, ItemId, Password, VersionMergeError};
    use maplit::hashset;
    use std::collections::HashSet;

    #[test]
    fn book_build() {
        let mut book = Book::new();
        let id0 = book.add(Item::new("Test 1")).unwrap();

        book.modify(id0, |it| {
            it.password = Some(Password::new("SecondPass123"));
        })
        .unwrap();

        let mut item1 = Item::new("Test 2");
        item1.password = Some(Password::new("TestPass456"));
        book.add(item1).unwrap();

        book.add(Item::new("Test 3")).unwrap();

        let mut items = book.items_metadata();
        items.sort_by_key(|(_, meta)| meta.created);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].0.name, "Test 1");
        assert_eq!(items[1].0.name, "Test 2");
        assert_eq!(items[2].0.name, "Test 3");
        assert!(items[0].0.password.is_some());
        assert!(items[1].0.password.is_some());
        assert!(items[2].0.password.is_none());
    }

    #[test]
    fn book_differ_index() {
        let mut book1 = Book::new();

        assert_eq!(book1.differ_index(&book1), None);

        book1.add(Item::new("Test 1")).unwrap();
        assert_eq!(book1.differ_index(&book1), None);

        let mut book2 = book1.clone();
        book2.add(Item::new("Test 2")).unwrap();
        assert_eq!(book1.differ_index(&book2), Some(2));

        book2.add(Item::new("Test 3")).unwrap();
        assert_eq!(book1.differ_index(&book2), Some(2));

        book2.remove("Test 3").unwrap();
        assert_eq!(book1.differ_index(&book2), Some(2));
    }

    #[test]
    fn book_merge_versions() -> Result<(), VersionMergeError> {
        let mut book1 = Book::new();
        let mut book2 = book1.clone();

        book1.add(Item::new("Test 1")).unwrap();
        book1.add(Item::new("Test 2")).unwrap();

        book2.add(Item::new("Test 3")).unwrap();

        let mut book3 = book2.clone();
        book3.add(Item::new("Test 4")).unwrap();

        assert_eq!(book1.item_ids().len(), 2);
        assert_eq!(book2.item_ids().len(), 1);
        assert_eq!(book3.item_ids().len(), 2);

        book3.remove("Test 3").unwrap();

        assert_eq!(book3.item_ids().len(), 1);

        let merged_12 = book1.merge_versions(&book2)?;
        assert_eq!(merged_12.item_ids().len(), 3);

        let merged_123 = merged_12.merge_versions(&book3)?;
        assert_eq!(
            merged_123
                .items()
                .iter()
                .map(|item| item.name.clone())
                .collect::<HashSet<_>>(),
            hashset!["Test 1".to_owned(), "Test 2".to_owned(), "Test 4".to_owned()]
        );

        let merged_1233 = merged_123.clone().merge_versions(&book3)?;
        let mut items_123 = merged_123.items_metadata();
        items_123.sort_by_key(|(_, meta)| meta.created);
        let mut items_1233 = merged_1233.items_metadata();
        items_1233.sort_by_key(|(_, meta)| meta.created);
        assert_eq!(items_123, items_1233);

        Ok(())
    }

    #[test]
    #[should_panic]
    fn book_remove_nonexistent() {
        let mut book = Book::new();
        book.remove("Nonexistent").unwrap();
    }
}
