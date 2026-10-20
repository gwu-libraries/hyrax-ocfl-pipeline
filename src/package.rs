use crate::resources::*;
use std::sync::mpsc;
use anyhow::Result;
use std::collections::HashMap;

pub fn package_resources(resources: mpsc::Receiver<OrmResource>, resources_to_ship: mpsc::Sender<OrmResource>) -> Result<()> {
    // works waiting to be packaged go here
    let mut work_staging: HashMap<String, OrmResource> = HashMap::new();
    // filesets waiting to be packaged with their works go here
    let mut file_set_staging: HashMap<String, OrmResource> = HashMap::new();
    // all other resources not yet packaged go here
    let mut staging: HashMap<String, OrmResource> = HashMap::new();

    for mut resource in resources {
        match &resource.model {
            HyraxModel::Work => {
                resource.pack_members(&mut staging);
                let id = resource.id.clone();
                let _ = work_staging.insert(id, resource);
            },
            HyraxModel::FileSet => {
                resource.pack_members(&mut staging);
                let id = resource.id.clone();
                let _ = file_set_staging.insert(id, resource);
            },
            _ => {
                if let Some(resource) = resource.pack_self(&mut work_staging) {
                    if let Some(resource) = resource.pack_self(&mut file_set_staging) {
                        staging.insert(resource.id.clone(), resource);
                    }
                }
            }
        }
        for work_resource in work_staging.values_mut() {
            work_resource.pack_members(&mut file_set_staging);
        }
        work_staging.extract_if(|_, resource| {
            resource.is_packed()
        }).for_each(|(_, resource)| {
            let result = resources_to_ship.send(resource);
            if let Err(err) = result {
                eprintln!("{:?}", err);
            }
        });

    }

    todo!("Returns some result")
}
