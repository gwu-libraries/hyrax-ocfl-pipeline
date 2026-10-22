use serde::{Deserialize};
use serde_json::json;

// wrapper around each resource
#[derive(Debug, Deserialize)]
pub struct OrmResource { // <'a> {
    data: String, //  all ORM data as String, heap-allocated, stored once per resource
    pub id: String, // unique resource ID
    refs: Option<Vec<String>>, /// other resource IDs pointed to by this one
    packed: Option<Vec<Box<OrmResource>>>, // child resources
    pub model: HyraxModel,
}

// label for matching on
// derive attributes allow us to use it also as the key in a hash
// TO DO: attributes to rename strings from JSON
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Deserialize)]
pub enum HyraxModel {
    Work,
    FileSet,
    Permissions,
    FileMetadata, // TO DO: add filename and type as inner struct
    Embargo,
}

impl HyraxModel {
    pub fn from_model_str(model: &str, work_types: &[&str]) -> Option<Self> {
        if work_types.contains(&model) {
            Some(HyraxModel::Work)
        } else {
            match model {
                "Hyrax::AccessControl" => Some(Self::Permissions),
                "Hyrax::FileMetadata" => Some(Self::FileMetadata),
                "Hyrax::FileSet" => Some(Self::FileSet),
                "Hyrax::Embargo" => Some(Self::Embargo),
                _ => None
            }
        }
    }
}

impl OrmResource {

    pub fn into_ocfl(self) -> (String, String) {
        (self.id, self.data)
    }

    pub fn from_row(id: String, data: String, refs: Option<Vec<String>>, model: HyraxModel) -> OrmResource {
        Self {
                    id,
                    data,
                    refs,
                    model,
                    packed: None,
                }
    }

    pub fn refs(&self) -> Option<&Vec<String>> {
        self.refs.as_ref()
    }

    pub fn check_for_child(&self, resource: &OrmResource) -> Option<bool> {
        // Permissions by default have no children; their refs point to parents
        if self.model == HyraxModel::Permissions {
            return None;
        }
        // No refs --> no children (except perhaps a Permission)
        let refs = self.refs()?;
        for _ref in refs {
            if _ref == resource.id.as_str() {
                return Some(true);
            }
        }
        Some(false) // has children, but this resource isn't one of them
    }


    pub fn pack_child_mut(&mut self, resource: OrmResource) {
        self.packed.get_or_insert_default().push(Box::new(resource));
    }

    pub fn pack_child(mut self, resource: OrmResource) -> Self {
        self.packed.get_or_insert_default().push(Box::new(resource));
        self
    }

    pub fn is_packed(&self) -> Option<bool> {
        let refs = self.refs()?; // if None, then automatically packed
        match self.model {
            HyraxModel::FileSet | HyraxModel::Work => {
                if let Some(packed) = self.packed.as_ref() {
                    Some(refs.len() == packed.len()) // for FileSet & Work, the number of children will include the permission object pointing to this resource, in addition to any other children
                } else {
                    Some(false) // nothing packed yet
                }
            },
            HyraxModel::Permissions => {
                Some(true) // permissions have no children, so packed by default
            },
            _ => { Some(true) } // Raise error? FileMetadata and Embargo should have no children
        }
    }

}

pub fn unnest_resources(mut root_resource: Box<OrmResource>) -> Vec<Box<OrmResource>> {
    let resources = root_resource.packed.take();
    let mut results: Vec<Box<OrmResource>> = vec![];
    if let Some(resources) = resources {
        for resource in resources.into_iter() {
            for nested in unnest_resources(resource) {
                results.push(nested);
            }
        }
    }
    results.push(root_resource);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_orm_json() {
        let json = r#"{"data": "",
            "id": "aaaa-bbbb-cccc-dddd",
            "refs": ["1111-2222-3333-4444", "5555-6666-7777-8888"],
            "model": "Work"}
            "#;
        let resource: OrmResource = serde_json::from_str(json).unwrap();
        assert_eq!(resource.id, "aaaa-bbbb-cccc-dddd".to_owned());
        assert_eq!(resource.refs, Some(vec!["1111-2222-3333-4444".to_owned(), "5555-6666-7777-8888".to_owned()]));

    }

    #[test]
    fn unnests_orm_resources() {

        let third = OrmResource {
            data: "".to_owned(),
            id: "3".to_owned(),
            refs: None,
            model: HyraxModel::Work,
            packed: None,
        };

        let second = OrmResource {
            data: "".to_owned(),
            id: "2".to_owned(),
            refs: None,
            model: HyraxModel::Work,
            packed: Some(vec![Box::new(third)]),
        };

        let first = OrmResource {
            data: "".to_owned(),
            id: "1".to_owned(),
            refs: None,
            model: HyraxModel::Work,
            packed: Some(vec![Box::new(second)]),
        };
        let unnested: Vec<String> = unnest_resources(Box::new(first)).iter().map(|resource| resource.id.to_owned()).collect();
        assert_eq!(unnested, vec!["3".to_owned(), "2".to_owned(), "1".to_owned()]);

    }
}
