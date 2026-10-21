use crate::resources::*;
use std::sync::mpsc;
use anyhow::Result;
use std::collections::HashMap;

pub struct Staging {
    resources: HashMap<String, OrmResource>, // maps each resource to its unique ID
    refs: HashMap<String, HashMap<HyraxModel, String>>, // maps each reference to a resource to the referrer, i.e., if a Work.member_ids contains a FileSet.id, refs[FileSet.id][HyraxModel::Work] = Work.id
}

impl Staging {

    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            refs: HashMap::new(),
        }
    }

    pub fn update_refs(&mut self, resource: &OrmResource) {
        if let Some(refs) = resource.refs() {
            for ref_id in refs {
                if let Some(_ref) = self.refs.get_mut(ref_id.as_str()) {
                    let _ = _ref.insert(resource.model, resource.id.clone());
                } else {
                    let mut inner_hash = HashMap::new();
                    let _ = inner_hash.insert(resource.model, resource.id.clone());
                    let _ = self.refs.insert(ref_id.clone(), inner_hash);
                }
            }
        }
    }

    pub fn stage_with_children(&mut self, mut resource: OrmResource) {
        if let Some(ids_to_remove) = resource.pack_children(&mut self.resources) {
            for id in ids_to_remove {
                self.refs.remove(id.as_str());
            }
        }
        if let Some(refs) = self.refs.get_mut(resource.id.as_str()) {
            if let Some(permission_id) = refs.remove(&HyraxModel::Permissions) {
                if let Some(permission) = self.resources.remove(permission_id.as_str()) {
                    resource.pack_one(permission);
                }

            }
        }
        if !resource.is_packed() {
            self.update_refs(&resource);
            let _ = self.resources.insert(resource.id.clone(), resource);
        } else {
            self.handle_packed(resource);
        }
    }

    fn handle_packed(&mut self, resource: OrmResource) {
        match &resource.model {
            HyraxModel::Work => {
                todo!("Send work")
            },
            HyraxModel::FileSet => {
                todo!("Pack into work")
            }
            _ => {}
        }
    }

    pub fn stage_no_children(&mut self, resource: OrmResource) {
        if let Some(refs) = self.refs.get_mut(resource.id.as_str()) {
            let mut parent_id: Option<String> = None;
            if refs.contains_key(&HyraxModel::Work) {
                parent_id = refs.remove(&HyraxModel::Work);

            } else {
                parent_id = refs.remove(&HyraxModel::FileSet);
            }
            if let Some(parent_id) = parent_id {
                let parent = self.resources.get_mut(parent_id.as_str());
                if let Some(parent) = parent {
                    parent.pack_one(resource);
                }
                let parent = self.resources.get(parent_id.as_str());
                if let Some(parent) = parent {
                    if parent.is_packed() {
                        if let Some(parent) = self.resources.remove(parent.id.as_str()) {
                            self.handle_packed(parent);

                        }
                    }
                }
            }
        }
    }
}
