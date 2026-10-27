use crate::resources::*;
use std::sync::mpsc;
use anyhow::Result;
use rocfl::ocfl::{OcflRepo, SpecVersion, RocflError, DigestAlgorithm};
use std::path::{Path, PathBuf};
use std::fs::{self, write, File};
use std::io::BufWriter;
use serde_json;
use std::thread;


/* TO DO: init from config */
pub fn init_repo(storage_root: PathBuf) -> Result<OcflRepo, RocflError> {
    let version = SpecVersion::Ocfl1_1;
    // Try first to initialize a new repo
    let repo_result = OcflRepo::init_fs_repo(&storage_root, None, version, None);
    match repo_result {
        Ok(repo) => Ok(repo),
        Err(err) => {
            // If the repo isn't empty, try to use it
            if let RocflError::IllegalState(msg) = &err {
                if msg.ends_with("must be empty") {
                    OcflRepo::fs_repo(&storage_root, None)
                } else {
                    Err(err)
                }
            } else {
                Err(err)
            }
        }
    }
}

/*
 * TO DO: confirm defaults
 * TO DO: remove hyphens from UUID?
 * Creates a new OCFL object in the provided repo, using the ID of the resource, which should correspond to a Hyrax::Work object. For the work and each resource nested under the work, persist the JSON metadata to a file in temporary storage.
 * This method consumes the resources, returning the paths to the files in temp storage.
 */
pub fn new_from_hyrax_metadata(work_resource: OrmResource, repo: &OcflRepo, path_to_tmp_root: &PathBuf) -> Result<(String, Vec<PathBuf>)> {
    let id = work_resource.id.clone(); // clone out of resource so that we can return it
    repo.create_object(id.as_str(), None, DigestAlgorithm::Sha512, "content", 0)?;
    // TO DO: move Boxing into staging method?
    let tmp_work_path = path_to_tmp_root.join(&work_resource.id);
    let resources = unnest_resources(Box::new(work_resource));
    fs::create_dir(&tmp_work_path)?;
    thread::scope(move |s| {
        let mut handles = vec![];
        for resource in resources {
            let (id, data) = resource.into_ocfl();
            let tmp_work_path_file = tmp_work_path.join(id).join(".json");
            let handle = s.spawn(|| -> Result<PathBuf> {
                create_temp_metadata_file(data, tmp_work_path_file)
            });
            handles.push(handle);
        }
        // Map to either a list of the successfully created paths, or bubble up the first error
        // If any files failed to save, then we need to log and abort
        // Verified that errors propagate: https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=f9bd713a7c348f17d163bf46c3f3ef87
        let results: Result<Vec<PathBuf>> = handles.into_iter().flat_map(|handle| handle.join()).collect::<Result<Vec<_>>>();
        match results {
            Ok(paths) => Ok((id, paths)),
            Err(err) => Err(err) // TO DO: annotate error with ID
        }
    })

}

/*
 * TO DO: This is needed because rocfl expects all files to pre-exist object creation. It would be preferable to stream the metadata directly into the OCFL staging storage, as opposed to persisting it outside the object first and then moving it
 */
fn create_temp_metadata_file(data: String, path_to_tmp_file: PathBuf) -> Result<PathBuf> {
    write(&path_to_tmp_file, data)?;
    Ok(path_to_tmp_file)
}
