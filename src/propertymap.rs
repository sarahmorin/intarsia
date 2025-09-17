use crate::types::{PropSetId, PropertySet};
/// PropertySet map is a BiMap struct that maps between property sets and unique IDs.
/// This allows us to efficiently reference and manage properties associated with expressions in the e-graph.
use bimap::BiMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

pub struct PropertySetMap<P: PropertySet>(BiMap<P, PropSetId>);

impl<P> PropertySetMap<P>
where
    P: PropertySet,
{
    /// Creates a new, empty PropertySetMap.
    pub fn new() -> Self {
        let mut props = BiMap::new();
        props.insert(P::bottom(), PropSetId(0)); // Insert bottom property with ID 0
        Self(props)
    }

    /// Inserts a property set into the map if it doesn't already exist,
    /// returning the corresponding unique ID.
    pub fn insert(&mut self, prop: &P) -> PropSetId {
        if let Some(id) = self.0.get_by_left(&prop) {
            *id
        } else {
            let new_id = PropSetId(self.0.len());
            self.0.insert(prop.clone(), new_id);
            new_id
        }
    }

    /// Retrieves a property set by its unique ID, if it exists.
    pub fn get_by_id(&self, id: &PropSetId) -> Option<&P> {
        self.0.get_by_right(id)
    }

    /// Retrieves the unique ID for a given property set, if it exists.
    pub fn get_by_propset(&self, prop_set: &P) -> Option<&PropSetId> {
        self.0.get_by_left(prop_set)
    }
}

impl<P> Display for PropertySetMap<P>
where
    P: PropertySet + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "PropertySetMap {{")?;
        for (prop, id) in self.0.iter() {
            writeln!(f, "  {} -> {}", prop, id)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Display;
    use std::hash::{Hash, Hasher};

    // Test property for testing
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct TestPropertySet {
        value: PropSetId,
    }

    impl Hash for TestPropertySet {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.value.hash(state);
        }
    }

    impl Display for TestPropertySet {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            write!(f, "TestProp({})", self.value)
        }
    }

    impl PropertySet for TestPropertySet {
        fn bottom() -> Self {
            TestPropertySet {
                value: PropSetId(0),
            }
        }
    }

    impl TestPropertySet {
        fn new(value: usize) -> Self {
            TestPropertySet {
                value: PropSetId(value),
            }
        }
    }

    #[test]
    fn test_new_property_map() {
        let prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        // Should have bottom property with ID 0
        assert_eq!(
            prop_map.get_by_id(&PropSetId(0)),
            Some(&TestPropertySet::bottom())
        );
        assert_eq!(
            prop_map.get_by_propset(&TestPropertySet::bottom()),
            Some(&PropSetId(0))
        );
    }

    #[test]
    fn test_insert_new_property() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        let prop1 = TestPropertySet::new(5);
        let id1 = prop_map.insert(&prop1);

        // Should get a new ID (1, since 0 is taken by bottom)
        assert_eq!(id1, PropSetId(1));
        assert_eq!(prop_map.get_by_id(&id1), Some(&prop1));
        assert_eq!(prop_map.get_by_propset(&prop1), Some(&id1));
    }

    #[test]
    fn test_insert_existing_property() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        let prop1 = TestPropertySet::new(5);
        let id1 = prop_map.insert(&prop1);
        let id2 = prop_map.insert(&prop1); // Insert same property again

        // Should return the same ID
        assert_eq!(id1, id2);
        assert_eq!(id1, PropSetId(1));
    }

    #[test]
    fn test_insert_multiple_properties() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        let prop1 = TestPropertySet::new(5);
        let prop2 = TestPropertySet::new(10);
        let prop3 = TestPropertySet::new(15);

        let id1 = prop_map.insert(&prop1);
        let id2 = prop_map.insert(&prop2);
        let id3 = prop_map.insert(&prop3);

        // Should get unique IDs
        assert_eq!(id1, PropSetId(1));
        assert_eq!(id2, PropSetId(2));
        assert_eq!(id3, PropSetId(3));

        // Should be able to retrieve all properties
        assert_eq!(prop_map.get_by_id(&id1), Some(&prop1));
        assert_eq!(prop_map.get_by_id(&id2), Some(&prop2));
        assert_eq!(prop_map.get_by_id(&id3), Some(&prop3));

        // Should be able to retrieve all IDs
        assert_eq!(prop_map.get_by_propset(&prop1), Some(&id1));
        assert_eq!(prop_map.get_by_propset(&prop2), Some(&id2));
        assert_eq!(prop_map.get_by_propset(&prop3), Some(&id3));
    }

    #[test]
    fn test_get_by_id_nonexistent() {
        let prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        // Should return None for non-existent ID
        assert_eq!(prop_map.get_by_id(&PropSetId(999)), None);
    }

    #[test]
    fn test_get_by_propset_nonexistent() {
        let prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        let nonexistent_prop = TestPropertySet::new(999);

        // Should return None for non-existent property
        assert_eq!(prop_map.get_by_propset(&nonexistent_prop), None);
    }

    #[test]
    fn test_bottom_property_always_present() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        // Insert some other properties
        let prop1 = TestPropertySet::new(5);
        let prop2 = TestPropertySet::new(10);
        prop_map.insert(&prop1);
        prop_map.insert(&prop2);

        // Bottom should still be accessible with ID 0
        assert_eq!(
            prop_map.get_by_id(&PropSetId(0)),
            Some(&TestPropertySet::bottom())
        );
        assert_eq!(
            prop_map.get_by_propset(&TestPropertySet::bottom()),
            Some(&PropSetId(0))
        );
    }

    #[test]
    fn test_display_implementation() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        let prop1 = TestPropertySet::new(5);
        let prop2 = TestPropertySet::new(10);
        prop_map.insert(&prop1);
        prop_map.insert(&prop2);

        let display_output = format!("{}", prop_map);

        // Should contain the property mappings
        assert!(display_output.contains("PropertySetMap {"));
        assert!(display_output.contains("TestProp(PropSetId(0)) -> PropSetId(0)"));
        assert!(display_output.contains("TestProp(PropSetId(5)) -> PropSetId(1)"));
        assert!(display_output.contains("TestProp(PropSetId(10)) -> PropSetId(2)"));
        assert!(display_output.contains("}"));
    }

    #[test]
    fn test_insert_bottom_property_explicitly() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        // Try to insert bottom property explicitly
        let bottom_id = prop_map.insert(&TestPropertySet::bottom());

        // Should return existing ID 0
        assert_eq!(bottom_id, PropSetId(0));
        assert_eq!(
            prop_map.get_by_id(&PropSetId(0)),
            Some(&TestPropertySet::bottom())
        );
    }

    #[test]
    fn test_sequential_id_assignment() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        // Insert properties and verify IDs are assigned sequentially
        let mut expected_id = PropSetId(1); // Start from 1 since 0 is bottom

        for value in 1..=10 {
            let prop = TestPropertySet::new(value);
            let assigned_id = prop_map.insert(&prop);
            assert_eq!(assigned_id, expected_id);
            expected_id = PropSetId(expected_id.as_usize() + 1);
        }
    }

    #[test]
    fn test_bidirectional_mapping_consistency() {
        let mut prop_map: PropertySetMap<TestPropertySet> = PropertySetMap::new();

        let properties = vec![
            TestPropertySet::new(1),
            TestPropertySet::new(5),
            TestPropertySet::new(10),
            TestPropertySet::new(20),
        ];

        let mut ids = Vec::new();

        // Insert all properties and collect IDs
        for prop in &properties {
            ids.push(prop_map.insert(prop));
        }

        // Verify bidirectional consistency
        for (i, prop) in properties.iter().enumerate() {
            let id = ids[i];

            // Forward mapping: property -> ID
            assert_eq!(prop_map.get_by_propset(prop), Some(&id));

            // Reverse mapping: ID -> property
            assert_eq!(prop_map.get_by_id(&id), Some(prop));
        }
    }
}
