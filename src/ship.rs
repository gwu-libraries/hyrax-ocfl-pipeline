use crate::resources::*;
use std::sync::mpsc;
use anyhow::Result;
use rocfl::ocfl::{OcflRepo, SpecVersion, RocflError, DigestAlgorithm};
use std::path::PathBuf;

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
 */
pub fn new_from_hyrax_metadata(work_resource: OrmResource, repo: &OcflRepo) -> Result<()> {

    let ocfl_object = repo.create_object(work_resource.id.as_str(), None, DigestAlgorithm::Sha512, "content", 0)?;
    // TO DO: move Boxing into staging method?
    let resources = unnest_resources(Box::new(work_resource));
    Ok(())


}

/*
 * TO DO: This is needed because rocfl expects all files to pre-exist object creation. It would be preferable to stream the metadata directly into the OCFL staging storage, as opposed to persisting it outside the object first and then moving it in
 */
fn create_temp_metadata_file(data: String, path_to_temp_storage: &PathBuf) -> Result<()> {

}
