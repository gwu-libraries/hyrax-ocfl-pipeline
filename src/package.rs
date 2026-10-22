use crate::resources::*;
use std::sync::mpsc;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Staging {
    resources: HashMap<HyraxModel, HashMap<String, usize>>, // maps the index of each resource in the staging list to its unique ID, organized by model type for more efficient searching
    staging_list: Vec<OrmResource>,

}

impl Staging {

    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            staging_list: vec![],
        }
    }

    fn indices_for_key(&self, resource_key: &HyraxModel) -> Vec<usize> {
        // no need to check resources against those of the same type
        let keys: Vec<HyraxModel> = self.resources.keys().copied().filter(|key| key != resource_key).collect();
        // Get indices to the resources of all other types in the list
        keys.iter().map(|key| {
            self.resources[key].values()
        }).flatten().copied().collect()
    }

    fn pack_children(&mut self, mut resource: OrmResource) -> Result<OrmResource> {

        let indices: Vec<usize> = self.indices_for_key(&resource.model).into_iter().filter(|idx| {
            let other = self.staging_list.get(*idx);
            if let Some(other) = other {
                let is_child = resource.check_for_child(other.id.as_str());
                match is_child {
                    None => false,
                    Some(result) => result,
                }
            } else {
                false
            }
        }).collect();
        for idx in indices {
            let other = self.staging_list.swap_remove(idx);
            resource.pack_child(other);
            if let Some(to_update) = self.staging_list.get(idx) {
                if let Some(resource_map) = self.resources.get_mut(&to_update.model) {
                    resource_map.entry(to_update.id.clone()).insert_entry(idx);
                }
            }
        }
        Ok(resource)
    }

    fn pack_if_parent_staged(&mut self, resource: OrmResource) -> Result<Option<OrmResource>> {

        // flag variable to avoid moving the resource out of the loop
        let mut to_pack: Option<usize> = None;
        // inspect each resource to see if the new resource is a child of one already staged
        for i in self.indices_for_key(&resource.model).into_iter() {
            if let Some(other) = self.staging_list.get(i) {
                match other.check_for_child(resource.id.as_str()) {
                    None => { },  // means this resource has no children
                    Some(is_child) => {
                        if is_child {
                            let _ = to_pack.insert(i);
                            break;
                        }
                    }
                }
            }
        }
        // if a parent was found, attach the child to the parent and return None --> nothing more to do with this resource
        if let Some(parent_idx) = to_pack {
            if let Some(other) = self.staging_list.get_mut(parent_idx) {
                other.pack_child(resource);
                return Ok(None)
            }
        }
        Ok(Some(resource))
    }


    fn stage_if_waiting(&mut self, resource: OrmResource) -> Result<()> {
        let resource_map = self.resources.entry(resource.model).or_insert_with(|| HashMap::new());
        resource_map.insert(resource.id.clone(), self.staging_list.len());
        self.staging_list.push(resource);
        Ok(())
    }

    pub fn stage(&mut self, resource: OrmResource) -> Result<()> {
        let packed = resource.is_packed();

        if packed.map_or(true, |r| r) {
                let resource = self.pack_if_parent_staged(resource)?;
                if let Some(resource) = resource {
                    let _ = self.stage_if_waiting(resource);
                }
        } else {
            let resource = self.pack_children(resource)?;
            if let Some(result) = resource.is_packed() {
                if result && resource.model != HyraxModel::Work {
                    self.stage(resource)?;
                } else {
                    let _ = self.stage_if_waiting(resource);
                }
            }
        }
        Ok(())
    }
}
