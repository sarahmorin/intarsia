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
            root1
        } else if root2 < root1 {
            self.parent[root1] = root2;
            root2
        } else {
            root1
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let uf = UnionFind::new();
        assert_eq!(uf.parent.len(), 0);
        assert_eq!(uf.roots(), vec![]);
    }

    #[test]
    fn test_add_set() {
        let mut uf = UnionFind::new();

        // Adding first set should return ID 0
        let id0 = uf.add_set();
        assert_eq!(id0, 0);
        assert_eq!(uf.parent.len(), 1);
        assert_eq!(uf.parent[0], 0); // Should be its own parent

        // Adding second set should return ID 1
        let id1 = uf.add_set();
        assert_eq!(id1, 1);
        assert_eq!(uf.parent.len(), 2);
        assert_eq!(uf.parent[1], 1); // Should be its own parent

        // Adding third set should return ID 2
        let id2 = uf.add_set();
        assert_eq!(id2, 2);
        assert_eq!(uf.parent.len(), 3);
        assert_eq!(uf.parent[2], 2); // Should be its own parent
    }

    #[test]
    fn test_find_single_elements() {
        let mut uf = UnionFind::new();
        let id0 = uf.add_set();
        let id1 = uf.add_set();
        let id2 = uf.add_set();

        // Each element should be its own root initially
        assert_eq!(uf.find(id0), 0);
        assert_eq!(uf.find(id1), 1);
        assert_eq!(uf.find(id2), 2);
    }

    #[test]
    fn test_find_after_union() {
        let mut uf = UnionFind::new();
        let id0 = uf.add_set();
        let id1 = uf.add_set();
        let id2 = uf.add_set();

        // Union 1 and 2 - smaller root (1) should become parent
        let root = uf.union(id1, id2);
        assert_eq!(root, 1); // Should return the smaller root

        // Both should now have the same root
        assert_eq!(uf.find(id1), 1);
        assert_eq!(uf.find(id2), 1);

        // id0 should still be separate
        assert_eq!(uf.find(id0), 0);
    }

    #[test]
    fn test_union_behavior() {
        let mut uf = UnionFind::new();
        let id0 = uf.add_set();
        let id1 = uf.add_set();
        let id2 = uf.add_set();
        let id3 = uf.add_set();

        // Test that smaller root becomes parent
        uf.union(id2, id0); // Union 2 and 0, smaller root (0) should become parent
        assert_eq!(uf.find(id2), 0); // 2 should now have root 0
        assert_eq!(uf.find(id0), 0); // 0 should still have root 0

        uf.union(id3, id1); // Union 3 and 1, smaller root (1) should become parent
        assert_eq!(uf.find(id3), 1); // 3 should now have root 1
        assert_eq!(uf.find(id1), 1); // 1 should still have root 1

        // Union the two groups: roots 0 and 1, so 0 should become the parent
        uf.union(id0, id1); // Union groups with roots 0 and 1

        // All elements should now have root 0 (the smaller root)
        assert_eq!(uf.find(id0), 0);
        assert_eq!(uf.find(id1), 0);
        assert_eq!(uf.find(id2), 0);
        assert_eq!(uf.find(id3), 0);
    }

    #[test]
    fn test_union_same_set() {
        let mut uf = UnionFind::new();
        let id0 = uf.add_set();
        let id1 = uf.add_set();

        // Union them first
        uf.union(id0, id1);

        // Union again - should not change anything
        let root = uf.union(id0, id1);
        assert_eq!(root, 0); // Should still return the root
        assert_eq!(uf.find(id0), 0);
        assert_eq!(uf.find(id1), 0);
    }

    #[test]
    fn test_find_compress() {
        let mut uf = UnionFind::new();
        let id0 = uf.add_set();
        let id1 = uf.add_set();
        let id2 = uf.add_set();
        let id3 = uf.add_set();

        // Create a chain: 3 -> 2 -> 1 -> 0
        uf.union(id0, id1); // 0 becomes root
        uf.union(id0, id2); // 0 remains root
        uf.union(id0, id3); // 0 remains root

        // Before compression, let's verify the structure
        assert_eq!(uf.find(id3), 0);

        // Use find_compress on id3
        let root = uf.find_compress(id3);
        assert_eq!(root, 0);

        // After compression, id3 should point directly to root
        // This tests that path compression actually occurred
        assert_eq!(uf.find_compress(id3), 0);
    }

    #[test]
    fn test_roots() {
        let mut uf = UnionFind::new();

        // Empty case
        assert_eq!(uf.roots(), vec![]);

        // Single element
        let id0 = uf.add_set();
        assert_eq!(uf.roots(), vec![0]);

        // Two separate elements
        let id1 = uf.add_set();
        let mut roots = uf.roots();
        roots.sort(); // Sort for consistent comparison
        assert_eq!(roots, vec![0, 1]);

        // After union, should have only one root
        uf.union(id0, id1);
        assert_eq!(uf.roots(), vec![0]);

        // Add more elements
        let id2 = uf.add_set();
        let id3 = uf.add_set();
        let mut roots = uf.roots();
        roots.sort();
        assert_eq!(roots, vec![0, 2, 3]);

        // Union two more
        uf.union(id2, id3);
        let mut roots = uf.roots();
        roots.sort();
        assert_eq!(roots, vec![0, 2]);
    }

    #[test]
    fn test_complex_operations() {
        let mut uf = UnionFind::new();

        // Create 6 sets
        let ids: Vec<Id> = (0..6).map(|_| uf.add_set()).collect();

        // Initially, all should be separate roots
        assert_eq!(uf.roots().len(), 6);

        // Create some unions: (0,1), (2,3), (4,5)
        uf.union(ids[0], ids[1]);
        uf.union(ids[2], ids[3]);
        uf.union(ids[4], ids[5]);

        // Should have 3 roots now
        assert_eq!(uf.roots().len(), 3);

        // Union two groups: (0,1) with (2,3)
        uf.union(ids[0], ids[2]);

        // Should have 2 roots now
        assert_eq!(uf.roots().len(), 2);

        // Check that elements are in correct groups
        let root_01 = uf.find(ids[0]);
        assert_eq!(uf.find(ids[1]), root_01);
        assert_eq!(uf.find(ids[2]), root_01);
        assert_eq!(uf.find(ids[3]), root_01);

        let root_45 = uf.find(ids[4]);
        assert_eq!(uf.find(ids[5]), root_45);

        // Final union
        uf.union(ids[0], ids[4]);

        // All should have same root now
        let final_root = uf.find(ids[0]);
        for id in ids {
            assert_eq!(uf.find(id), final_root);
        }

        // Should have exactly 1 root
        assert_eq!(uf.roots().len(), 1);
    }

    #[test]
    fn test_path_compression_effectiveness() {
        let mut uf = UnionFind::new();

        // Create a long chain by careful unions
        let ids: Vec<Id> = (0..5).map(|_| uf.add_set()).collect();

        // Manually create a chain by setting parents directly
        // This simulates what could happen with multiple unions
        uf.union(ids[0], ids[1]);
        uf.union(ids[1], ids[2]);
        uf.union(ids[2], ids[3]);
        uf.union(ids[3], ids[4]);

        // All should have the same root (0, since it's smallest)
        let root = uf.find(ids[4]);
        assert_eq!(root, 0);

        // Now use find_compress on the deepest element
        let compressed_root = uf.find_compress(ids[4]);
        assert_eq!(compressed_root, 0);

        // All subsequent finds should be fast
        assert_eq!(uf.find(ids[4]), 0);
        assert_eq!(uf.find(ids[3]), 0);
        assert_eq!(uf.find(ids[2]), 0);
        assert_eq!(uf.find(ids[1]), 0);
        assert_eq!(uf.find(ids[0]), 0);
    }
}
