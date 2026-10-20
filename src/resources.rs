use std::collections::HashMap;

// wrapper around each resource
pub struct OrmResource { // <'a> {
    data: String, //  all ORM data as String, heap-allocated, stored once per resource
    pub id: String, // unique resource ID
    refs: Vec<String>, //<&'a str>, // other resource IDs pointed to by this one -- difficulty using &str because we need to put in a Box...
    packed: Option<Vec<Box<OrmResource>>>, //<'a>>>>,
    pub model: HyraxModel,
}

// label for matching on
pub enum HyraxModel {
    Work,
    FileSet,
    Permissions,
    FileMetadata,
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

impl OrmResource { //<'_> {

    pub fn into_ocfl(self) -> (String, String) {
        (self.id, self.data)
    }

    pub fn from_row(id: String, data: String, refs: Vec<String>, model: HyraxModel) -> OrmResource {
        Self {
                    id,
                    data,
                    refs,
                    model,
                    packed: None,
                }
    }

    pub fn pack_members(&mut self, resources: &mut HashMap<String, OrmResource>)  {
        for id in self.refs.iter() {
           if let Some(resource) = resources.remove(id.as_str()) {
               if resource.is_packed() {
                   self.packed.get_or_insert_default().push(Box::new(resource));
               }
            }
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
