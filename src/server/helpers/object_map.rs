use super::*;
use bimap::BiHashMap;

pub type ObjectId = u64;

pub struct ObjectMap {
    map: BiHashMap<EntityKey, ObjectId>,
    next_id: ObjectId,
}

impl ObjectMap {
    pub fn new() -> Self {
        Self {
            map: BiHashMap::new(),
            next_id: 1,
        }
    }

    pub fn register_entity(&mut self, entity: EntityKey) -> ObjectId {
        let id = self.next_id;
        self.next_id += 1;
        if self.map.insert_no_overwrite(entity, id).is_err() {
            panic!("{:?} already in the bimap", entity);
        }
        id
    }

    pub fn remove_entity(&mut self, entity: EntityKey) -> Option<ObjectId> {
        self.map.remove_by_left(&entity).map(|(_, o)| o)
    }

    pub fn get_object(&self, entity: EntityKey) -> Option<ObjectId> {
        self.map.get_by_left(&entity).cloned()
    }

    pub fn get_entity(&self, object: ObjectId) -> Option<EntityKey> {
        self.map.get_by_right(&object).cloned()
    }

    /// Returns a "resolved" encodable if needed, or None if the unresolved encodable is fine
    /// Entities and collections containing them need to be resolved to object IDs
    pub fn resolve<'a>(&mut self, unresolved_value: &'a Encodable) -> Option<Encodable> {
        match unresolved_value {
            // if the encodable is an entity, we need to look up the connection-specific object ID
            Encodable::Entity(entity) => {
                Some(Encodable::Integer(match self.map.get_by_left(entity) {
                    Some(o) => *o,
                    None => self.register_entity(*entity),
                } as i64))
            }
            // if this encodable is a list, it could contain entities that should be resolved
            Encodable::List(list) => {
                // Search throug the list, looking for an element that need to be resolved
                match list
                    .iter()
                    .enumerate()
                    .find_map(|(i, element)| self.resolve(element).map(|resolved| (i, resolved)))
                {
                    // if we find one
                    Some((i, first_resolved)) => {
                        // clone the first part of the vec that didn't have any resolvable elements
                        let first_part = list.iter().take(i).cloned();
                        // insert the element we resolved
                        let with_first_resolved = first_part.chain(std::iter::once(first_resolved));
                        // and resolve or clone the rest of the vec as needed
                        let with_rest =
                            with_first_resolved.chain(list[i + 1..].iter().map(|element| {
                                self.resolve(element).unwrap_or_else(|| element.clone())
                            }));
                        Some(Encodable::List(with_rest.collect()))
                    }
                    // otherwise, no elements need to be resolved
                    None => None,
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod objects_tests {
    use super::*;

    #[test]
    fn registered_entities_and_objects_can_be_looked_up() {
        let mut map = ObjectMap::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.register_entity(*entity))
            .collect();
        assert_eq!(map.get_entity(o[0]), Some(e[0]));
        assert_eq!(map.get_object(e[0]), Some(o[0]));
        assert_eq!(map.get_object(e[1]), Some(o[1]));
        assert_eq!(map.get_entity(o[1]), Some(e[1]));
    }

    #[test]
    fn object_ids_count_up_from_1() {
        let mut map = ObjectMap::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.register_entity(*entity))
            .collect();
        assert_eq!(o[0], 1);
        assert_eq!(o[1], 2);
        assert!(map.get_entity(0).is_none());
        assert!(map.get_entity(1).is_some());
        assert!(map.get_entity(2).is_some());
        assert!(map.get_entity(3).is_none());
    }

    #[test]
    fn nonexistant_entities_and_objects_return_null() {
        let mut map = ObjectMap::new();
        let e = mock_keys(2);
        let o = 47;
        assert_eq!(map.get_entity(o), None);
        assert_eq!(map.get_object(e[0]), None);
        map.register_entity(e[0]);
        assert_eq!(map.get_entity(o), None);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    fn correct_object_removed() {
        let mut map = ObjectMap::new();
        let e = mock_keys(3);
        map.register_entity(e[0]);
        let o = map.register_entity(e[1]);
        assert_eq!(map.remove_entity(e[2]), None);
        assert_eq!(map.remove_entity(e[1]), Some(o));
        assert_eq!(map.remove_entity(e[1]), None);
    }

    #[test]
    fn object_and_entity_null_after_removal() {
        let mut map = ObjectMap::new();
        let e = mock_keys(2);
        let o: Vec<ObjectId> = e
            .iter()
            .map(|entity| map.register_entity(*entity))
            .collect();
        map.remove_entity(e[1]);
        assert_eq!(map.get_entity(o[1]), None);
        assert_eq!(map.get_object(e[1]), None);
    }

    #[test]
    #[should_panic]
    fn panics_if_same_entity_is_registered_twice() {
        let mut map = ObjectMap::new();
        let e = mock_keys(1);
        map.register_entity(e[0]);
        map.register_entity(e[0]);
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::Encodable::*;
    use super::*;

    fn map_entities_objects() -> (ObjectMap, Vec<EntityKey>, Vec<ObjectId>) {
        let mut map = ObjectMap::new();
        let e = mock_keys(3);
        let o = e
            .iter()
            .map(|entity| map.register_entity(*entity))
            .collect();
        (map, e, o)
    }

    #[test]
    fn resolves_scaler_without_changing() {
        let (mut map, _, _) = map_entities_objects();
        assert_eq!(map.resolve(&Scaler(12.5)), None);
    }

    #[test]
    fn resolves_list_without_changing() {
        let (mut map, _, _) = map_entities_objects();
        assert_eq!(map.resolve(&List(vec![Integer(7), Integer(12)])), None);
    }

    #[test]
    fn resolves_entity_to_object_id() {
        let (mut map, e, o) = map_entities_objects();
        assert_eq!(map.resolve(&Entity(e[0])), Some(Integer(o[0] as i64)));
        assert_eq!(map.resolve(&Entity(e[1])), Some(Integer(o[1] as i64)));
    }

    #[test]
    fn resolves_list_of_entities_to_object_ids() {
        let (mut map, e, o) = map_entities_objects();
        let original = List(e.iter().map(|e| Entity(*e)).collect());
        let resolved = List(o.iter().map(|o| Integer(*o as i64)).collect());
        assert_eq!(map.resolve(&original), Some(resolved));
    }

    #[test]
    fn resolves_list_with_entity_and_other_stuff() {
        let (mut map, e, o) = map_entities_objects();
        let original = List(vec![Integer(42), Entity(e[0]), Null]);
        let resolved = List(vec![Integer(42), Integer(o[0] as i64), Null]);
        assert_eq!(map.resolve(&original), Some(resolved));
    }

    #[test]
    fn resolves_nested_lists_with_entities() {
        let (mut map, e, o) = map_entities_objects();
        let original = List(vec![
            List(vec![List(vec![Entity(e[1]), Entity(e[2]), Null])]),
            Integer(42),
            Entity(e[0]),
        ]);
        let resolved = List(vec![
            List(vec![List(vec![
                Integer(o[1] as i64),
                Integer(o[2] as i64),
                Null,
            ])]),
            Integer(42),
            Integer(o[0] as i64),
        ]);
        assert_eq!(map.resolve(&original), Some(resolved));
    }
}
