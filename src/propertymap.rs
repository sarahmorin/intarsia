use crate::types::{Id, Property};
/// Property map is a BiMap struct that maps between property sets and unique IDs.
/// This allows us to efficiently reference and manage properties associated with expressions in the e-graph.
use bimap::BiMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

pub struct PropertyMap<P>
where
    P: Property,
{
    /// Maps property sets to unique IDs
    props: BiMap<P, Id>,
}

impl<P> PropertyMap<P>
where
    P: Property,
{
    /// Creates a new, empty PropertyMap
    pub fn new() -> Self {
        let mut props = BiMap::new();
        props.insert(P::bottom(), 0); // Insert bottom property with ID 0
        Self { props }
    }

    /// Inserts a property set into the map if it doesn't already exist,
    /// returning the corresponding unique ID.
    pub fn insert(&mut self, prop: &P) -> Id {
        if let Some(id) = self.props.get_by_left(&prop) {
            *id
        } else {
            let new_id = self.props.len();
            self.props.insert(prop.clone(), new_id);
            new_id
        }
    }

    /// Retrieves a property set by its unique ID, if it exists.
    pub fn get_by_id(&self, id: &Id) -> Option<&P> {
        self.props.get_by_right(id)
    }

    /// Retrieves the unique ID for a given property set, if it exists.
    pub fn get_by_props(&self, prop_set: &P) -> Option<&Id> {
        self.props.get_by_left(prop_set)
    }
}

impl<P> Display for PropertyMap<P>
where
    P: Property + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "PropertyMap {{")?;
        for (prop, id) in self.props.iter() {
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
    struct TestProperty {
        value: usize,
    }

    impl Hash for TestProperty {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.value.hash(state);
        }
    }

    impl Display for TestProperty {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            write!(f, "TestProp({})", self.value)
        }
    }

    impl Property for TestProperty {
        fn bottom() -> Self {
            TestProperty { value: 0 }
        }

        fn contains(&self, other: &Self) -> bool {
            self.value >= other.value
        }
    }

    impl TestProperty {
        fn new(value: usize) -> Self {
            TestProperty { value }
        }
    }

    #[test]
    fn test_new_property_map() {
        let prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        // Should have bottom property with ID 0
        assert_eq!(prop_map.get_by_id(&0), Some(&TestProperty::bottom()));
        assert_eq!(prop_map.get_by_props(&TestProperty::bottom()), Some(&0));
    }

    #[test]
    fn test_insert_new_property() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        let prop1 = TestProperty::new(5);
        let id1 = prop_map.insert(&prop1);

        // Should get a new ID (1, since 0 is taken by bottom)
        assert_eq!(id1, 1);
        assert_eq!(prop_map.get_by_id(&id1), Some(&prop1));
        assert_eq!(prop_map.get_by_props(&prop1), Some(&id1));
    }

    #[test]
    fn test_insert_existing_property() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        let prop1 = TestProperty::new(5);
        let id1 = prop_map.insert(&prop1);
        let id2 = prop_map.insert(&prop1); // Insert same property again

        // Should return the same ID
        assert_eq!(id1, id2);
        assert_eq!(id1, 1);
    }

    #[test]
    fn test_insert_multiple_properties() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        let prop1 = TestProperty::new(5);
        let prop2 = TestProperty::new(10);
        let prop3 = TestProperty::new(15);

        let id1 = prop_map.insert(&prop1);
        let id2 = prop_map.insert(&prop2);
        let id3 = prop_map.insert(&prop3);

        // Should get unique IDs
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);

        // Should be able to retrieve all properties
        assert_eq!(prop_map.get_by_id(&id1), Some(&prop1));
        assert_eq!(prop_map.get_by_id(&id2), Some(&prop2));
        assert_eq!(prop_map.get_by_id(&id3), Some(&prop3));

        // Should be able to retrieve all IDs
        assert_eq!(prop_map.get_by_props(&prop1), Some(&id1));
        assert_eq!(prop_map.get_by_props(&prop2), Some(&id2));
        assert_eq!(prop_map.get_by_props(&prop3), Some(&id3));
    }

    #[test]
    fn test_get_by_id_nonexistent() {
        let prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        // Should return None for non-existent ID
        assert_eq!(prop_map.get_by_id(&999), None);
    }

    #[test]
    fn test_get_by_props_nonexistent() {
        let prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        let nonexistent_prop = TestProperty::new(999);

        // Should return None for non-existent property
        assert_eq!(prop_map.get_by_props(&nonexistent_prop), None);
    }

    #[test]
    fn test_bottom_property_always_present() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        // Insert some other properties
        let prop1 = TestProperty::new(5);
        let prop2 = TestProperty::new(10);
        prop_map.insert(&prop1);
        prop_map.insert(&prop2);

        // Bottom should still be accessible with ID 0
        assert_eq!(prop_map.get_by_id(&0), Some(&TestProperty::bottom()));
        assert_eq!(prop_map.get_by_props(&TestProperty::bottom()), Some(&0));
    }

    #[test]
    fn test_display_implementation() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        let prop1 = TestProperty::new(5);
        let prop2 = TestProperty::new(10);
        prop_map.insert(&prop1);
        prop_map.insert(&prop2);

        let display_output = format!("{}", prop_map);

        // Should contain the property mappings
        assert!(display_output.contains("PropertyMap {"));
        assert!(display_output.contains("TestProp(0) -> 0"));
        assert!(display_output.contains("TestProp(5) -> 1"));
        assert!(display_output.contains("TestProp(10) -> 2"));
        assert!(display_output.contains("}"));
    }

    #[test]
    fn test_insert_bottom_property_explicitly() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        // Try to insert bottom property explicitly
        let bottom_id = prop_map.insert(&TestProperty::bottom());

        // Should return existing ID 0
        assert_eq!(bottom_id, 0);
        assert_eq!(prop_map.get_by_id(&0), Some(&TestProperty::bottom()));
    }

    #[test]
    fn test_sequential_id_assignment() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        // Insert properties and verify IDs are assigned sequentially
        let mut expected_id = 1; // Start from 1 since 0 is bottom

        for value in 1..=10 {
            let prop = TestProperty::new(value);
            let assigned_id = prop_map.insert(&prop);
            assert_eq!(assigned_id, expected_id);
            expected_id += 1;
        }
    }

    #[test]
    fn test_bidirectional_mapping_consistency() {
        let mut prop_map: PropertyMap<TestProperty> = PropertyMap::new();

        let properties = vec![
            TestProperty::new(1),
            TestProperty::new(5),
            TestProperty::new(10),
            TestProperty::new(20),
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
            assert_eq!(prop_map.get_by_props(prop), Some(&id));

            // Reverse mapping: ID -> property
            assert_eq!(prop_map.get_by_id(&id), Some(prop));
        }
    }
}
