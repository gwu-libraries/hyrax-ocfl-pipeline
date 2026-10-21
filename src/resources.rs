use std::collections::HashMap;

// wrapper around each resource
pub struct OrmResource { // <'a> {
    data: String, //  all ORM data as String, heap-allocated, stored once per resource
    pub id: String, // unique resource ID
    refs: Option<Vec<String>>, /// other resource IDs pointed to by this one
    packed: Option<Vec<Box<OrmResource>>>, // child resources
    pub model: HyraxModel,
}

// label for matching on
// derive attributes allow us to use it also as the key in a hash
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum HyraxModel {
    Work,
    FileSet,
    Permissions,
    FileMetadata, // TO DO: add filename and type as inner struct
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

    /* Scans the provided hash for references to resources that should be included as children of self. Returns a list of keys to remove from the hash of references where these refer to children that are now packed into self.
     *
     * TO DO -> Refactor to be able to delegate to self.pack_one(resource) without holding an immutable reference to self -->  return IDs to remove, and let Staging code handle the removal?
     */
    pub fn pack_children(&mut self, resources: &mut HashMap<String, OrmResource>) -> Option<Vec<String>>  {
        let mut refs_to_remove: Vec<String> = vec![];
        if let Some(ids) = self.refs.as_ref() {
            for id in ids.iter() {
               if let Some(resource) = resources.get(id.as_str()) {
                   if resource.is_packed() {
                       if let Some((id, resource)) = resources.remove_entry(id.as_str()) {
                           //self.pack_one(resource);
                           self.packed.get_or_insert_default().push(Box::new(resource));
                           refs_to_remove.push(id);
                       }
                   }
                }
            }
        }
        if refs_to_remove.len() > 0 {
            Some(refs_to_remove)
        } else {
            None
        }
    }

    pub fn pack_one(&mut self, resource: OrmResource) {
        self.packed.get_or_insert_default().push(Box::new(resource));
    }

    pub fn pack_self(self, resources: &mut HashMap<String, OrmResource>) -> Option<Self> {
        for other in resources.values_mut() {
            for id in other.refs.iter() {
                if self.id == *id {
                    other.pack_one(self);
                    return None;
                }
            }
        }
        return Some(self)
    }

    pub fn is_packed(&self) -> bool {
        match self.model {
            HyraxModel::Work | HyraxModel::FileSet =>
                self.packed.as_ref().filter(|p| p.len() == self.refs.len()).is_some(),
            _ => true
        }

    }
 }
