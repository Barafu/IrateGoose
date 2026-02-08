#![allow(dead_code)]
use crate::file_manager::WavFileData;
use std::collections::HashMap;

/// Indexed storage for WAV file data with fast lookup by checksum.
///
/// Maintains a vector of `WavFileData` items and a hash map from non‑zero checksums
/// to their positions in the vector. Zero checksums are not indexed.
#[derive(Clone, Default)]
pub struct WavFileIndex {
    items: Vec<WavFileData>,
    checksum_index: HashMap<u64, usize>,
}

impl WavFileIndex {
    /// Creates an empty `WavFileIndex`.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            checksum_index: HashMap::new(),
        }
    }

    /// Creates a `WavFileIndex` from an existing vector of `WavFileData`.
    ///
    /// Takes ownership of the vector and builds the checksum index.
    /// Items are kept in the order they appear in the vector.
    /// If a duplicate non‑zero checksum appears, the later item's position overwrites the earlier one.
    pub fn from_vec(items: Vec<WavFileData>) -> Self {
        let mut checksum_index = HashMap::new();
        for (idx, item) in items.iter().enumerate() {
            if item.checksum != 0 {
                checksum_index.insert(item.checksum, idx);
            }
        }
        Self {
            items,
            checksum_index,
        }
    }

    /// Removes all stored items and clears the index.
    pub fn clear(&mut self) {
        self.items.clear();
        self.checksum_index.clear();
    }

    /// Adds a `WavFileData` item to the index.
    ///
    /// The item is appended to the internal vector. If its checksum is non‑zero,
    /// the checksum is inserted into the index, mapping to the item’s position.
    /// If a duplicate non‑zero checksum already exists, the previous mapping is
    /// overwritten (duplicates are not expected in normal operation).
    pub fn add(&mut self, item: WavFileData) {
        let idx = self.items.len();
        self.items.push(item);
        if self.items[idx].checksum != 0 {
            self.checksum_index.insert(self.items[idx].checksum, idx);
        }
    }

    /// Returns the number of stored items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns a reference to the item at the given index, if it exists.
    pub fn get_by_index(&self, index: usize) -> Option<&WavFileData> {
        self.items.get(index)
    }

    /// Returns a reference to the item with the given checksum, if it exists.
    ///
    /// this method returns `None` for `checksum == 0`.
    pub fn get_by_checksum(&self, checksum: u64) -> Option<&WavFileData> {
        if checksum == 0 {
            return None;
        }
        self.checksum_index
            .get(&checksum)
            .and_then(|&idx| self.items.get(idx))
    }

    /// Returns an iterator over the stored items.
    pub fn iter(&self) -> std::slice::Iter<'_, WavFileData> {
        self.items.iter()
    }

    /// Creates a new `WavFileIndex` containing clones of items that satisfy the predicate.
    ///
    /// The predicate is called with a reference to each item; if it returns `true`,
    /// the item is cloned and added to the new index. The order of items is preserved.
    pub fn filtered_clone<P>(&self, predicate: P) -> Self
    where
        P: FnMut(&&WavFileData) -> bool,
    {
        let filtered_data: Vec<WavFileData> = self.items
            .iter()
            .filter(predicate)
            .cloned()
            .collect();
        let mut new_index = Self::from_vec(filtered_data);
        new_index.shrink_to_fit();
        new_index
    }

    /// Reduces the memory usage after all data has been filled.
    pub fn shrink_to_fit(&mut self) {
        self.items.shrink_to_fit();
        self.checksum_index.shrink_to_fit();
    }
}

impl From<Vec<WavFileData>> for WavFileIndex {
    fn from(items: Vec<WavFileData>) -> Self {
        Self::from_vec(items)
    }
}
