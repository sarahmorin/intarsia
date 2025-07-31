/// UnionFind
/// Simple data structure to hold equivalence classes.
use crate::types::Id;

#[derive(Debug, Clone)]
pub struct UnionFind {
    parent: Vec<Id>,
}

#[allow(dead_code)] // FIXME: Remove
impl UnionFind {
    /// Create a new UnionFind
    pub fn new() -> Self {
        Self { parent: Vec::new() }
    }

    /// Add a new set
    pub fn add_set(&mut self) -> Id {
        let index = self.parent.len();
        self.parent.push(index);
        index
    }

    /// Find the root of the set containing `id`.
    pub fn find(&self, id: Id) -> Id {
        let mut curr_id = id;
        while curr_id != self.parent[curr_id] {
            curr_id = self.parent[curr_id];
        }
        curr_id
    }

    /// Find the root of the set containing `x` with path compression.
    pub fn find_compress(&mut self, id: Id) -> Id {
        // Find the canonical representative of the set containing `id` with path compression
        let mut curr_id = id;
        while curr_id != self.parent[curr_id] {
            let parent = self.parent[curr_id];
            self.parent[curr_id] = self.parent[parent]; // Path compression
            curr_id = parent;
        }
        curr_id
    }

    /// Union two sets containing `id1` and `id2` and return the root id of the new set.
    /// The set with the smaller root id becomes the parent of the other.
    pub fn union(&mut self, id1: Id, id2: Id) -> Id {
        // TODO: Do we need to actually do the find, or should we just assume the caller
        // already did a find?
        let root1 = self.find(id1);
        let root2 = self.find(id2);

        if root1 < root2 {
            self.parent[root2] = root1;
        } else if root2 < root1 {
            self.parent[root1] = root2;
        }
        root1
    }

    /// Get list of all root elements.
    pub fn roots(&self) -> Vec<Id> {
        let mut roots = Vec::new();
        for (i, &parent) in self.parent.iter().enumerate() {
            if i == parent {
                roots.push(i);
            }
        }
        roots
    }
}
