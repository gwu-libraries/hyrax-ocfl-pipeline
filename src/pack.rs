use crate::resources::*;
use std::sync::mpsc;
use anyhow::Result;

#[derive(Debug)]
pub struct Staging {
    staging_list: Vec<OrmResource>,

}

impl Staging {

    pub fn new() -> Self {
        Self {
            staging_list: vec![],
        }
    }

    fn pack_children(&mut self, mut resource: OrmResource) -> Result<OrmResource> {
        let children: Vec<OrmResource> = self.staging_list.extract_if(.., |staged_resource| {
            resource.check_for_child(staged_resource)
                .unwrap_or(false) && staged_resource.is_packed().unwrap_or(true)
        }).collect();

        for child in children {
            resource.pack_child_mut(child);
        }
        Ok(resource)
    }

    fn pack_into_parent(&mut self, resource: OrmResource) -> Result<Option<OrmResource>> {

        for staged_resource in self.staging_list.iter_mut() {
            if staged_resource.check_for_child(&resource).unwrap_or(false) {
                staged_resource.pack_child_mut(resource);
                return Ok(None)
            }
        }
        Ok(Some(resource))
    }

    pub fn stage_resource(&mut self, resource: OrmResource) -> Result<()> {
        // if it has no children, this will be None
        // TO DO: need logic for permissions
        if resource.is_packed().unwrap_or(true) {
            let to_stage = self.pack_into_parent(resource)?;
            // If its parent is not already staged, waiting on parent; stage it
            if let Some(resource) = to_stage {
                self.staging_list.push(resource);
            }
        } else {
            // Not packed => may have staged children
            let resource = self.pack_children(resource)?;
            // If still waiting on more children, stage it
            if !resource.is_packed().unwrap_or(true) {
                self.staging_list.push(resource);
            } else {
                // otherwise, we can try to pack it into its parent
                return self.stage_resource(resource);
            }
        }
        Ok(())
    }

    pub fn update_staging_and_ship(&mut self) -> Result<()> {
        let unpacked: Vec<OrmResource> = self.staging_list.extract_if(.., |resource| {
            !resource.is_packed().unwrap_or(true)
        }).collect();
        for resource in unpacked.into_iter() {
            let _ = self.stage_resource(resource);
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{BufRead, BufReader}, path::PathBuf, fs::File};
    use std::collections::HashSet;

    fn load_test_data(filename: &str) -> Vec<OrmResource> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(format!("tests/testing_data/{}", filename));
        let file = File::open(d).expect(&format!("Couldn't open {}", filename));
        let resources: Vec<OrmResource> = BufReader::new(file)
                                                    .lines()
                                                    .map(|line| {
                                                        let line = line.unwrap();
                                                        serde_json::from_str(&line).expect(&format!("Couldn't parse line: {}", line))
                                                    }).collect();
        resources
    }

    #[test]
    fn packs_one_set() {

        let resources = load_test_data("orm_resources_one_set.jsonl");
        let mut staging = Staging::new();
        for resource in resources {
            staging.stage_resource(resource).unwrap();
        }
        let _ = staging.update_staging_and_ship();
        assert!(staging.staging_list.len() == 1);
        let work = staging.staging_list.pop().unwrap();
        assert_eq!(work.model, HyraxModel::Work);
        let unpacked = unnest_resources(Box::new(work));
        assert_eq!(unpacked.len(), 11);
        let models: HashSet<HyraxModel> = unpacked.iter().map(|resource| resource.model).collect();
        let mut should_have = HashSet::new();
        for var in [HyraxModel::Embargo, HyraxModel::FileSet, HyraxModel::Work, HyraxModel::FileMetadata, HyraxModel::Permissions] {
            should_have.insert(var);
        }
        assert_eq!(models, should_have);
    }

    #[test]
    fn packs_many() {
        let resources = load_test_data("orm_resources_multiple.jsonl");
        let mut staging = Staging::new();
        for resource in resources {
            staging.stage_resource(resource).unwrap();
        }
        let _ = staging.update_staging_and_ship();
        // Ensure everything is packed up at the Work level
        assert!(staging.staging_list.len() == 5);
        for work in staging.staging_list {
            assert_eq!(work.model, HyraxModel::Work);
            let unpacked = unnest_resources(Box::new(work));
            // ensure all resources  are grouped together as they should be
            // the test resources are created so that each group has a unique label in the data field
            let tag: HashSet<String> = unpacked.into_iter().map(|resource| {
                let data = resource.into_ocfl();
                data.1
            }).collect();
            assert_eq!(tag.len(), 1);
        }
    }

}
