use sqlx::FromRow;
use uuid::Uuid;
use serde_json::{self, Value};
use chrono::NaiveDateTime;
use crate::resources::{HyraxModel, OrmResource};
use serde::Deserialize;
use anyhow::{anyhow, Result};

/*
 * Deserializes a row from the Hyrax orm_resources table, corresponding to a Hyrax object of some type to be stored in OCFL
 */
#[derive(FromRow)]
#[derive(Debug)]
pub struct OrmResourceRow {
    pub id: Uuid,
    pub metadata: Value,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    pub internal_resource: String,
    lock_version: i32,
}

/*
 * Allow for minimal structured deserialization of the metadata field expressing the relation of works to filesets, filesets to files, and permissions to works and files.
 */
#[derive(Deserialize, Debug)]
struct Member {
    id: String,
}

#[derive(Deserialize)]
struct Members {
    member_ids: Vec<Member>
}

#[derive(Deserialize)]
struct Embargo {
    embargo_id: Option<[Member; 1]>
}

#[derive(Deserialize)]
#[serde(untagged)] // untagged, because these variants are just wrappers for two different types
enum FileIds {
    FileIdStrings(Vec<String>),
    FileIdMembers(Vec<Member>)
}

#[derive(Deserialize)]
struct Files {
    file_ids: FileIds
}

#[derive(Deserialize)]
struct AccessTo {
    access_to :Vec<Member>
}

fn extract_embargo(metadata: &Value) -> Result<Option<String>> {
    let with_embargo = Embargo::deserialize(metadata)?;
    if let Some([embargo]) = with_embargo.embargo_id {
        Ok(Some(embargo.id))
    } else {
        Ok(None)
    }

}

fn extract_file_ids(file_ids: FileIds) -> Result<Vec<String>> {
    match file_ids {
        FileIds::FileIdStrings(id_strings) => Ok(id_strings),
        FileIds::FileIdMembers(members) => {

            let ids = members.into_iter().map(|member| member.id).collect::<Vec<String>>();
            Ok(ids)
        }
    }
}

impl OrmResourceRow {
    pub fn to_orm_resource(self, work_types: &[&str]) -> Result<OrmResource> {
        if let Some(model) = HyraxModel::from_model_str(&self.internal_resource, work_types) {
            let refs: Option<Vec<String>> = match &model {
                HyraxModel::Work  => {
                    let with_members = Members::deserialize(&self.metadata)?;
                    let mut members = with_members.member_ids.into_iter().map(|member| member.id).collect::<Vec<String>>();
                    if let Some(embargo_id) = extract_embargo(&self.metadata)? {
                        members.push(embargo_id)
                    }
                    Some(members)
                },
                HyraxModel::FileSet => {
                    let with_files = Files::deserialize(&self.metadata)?;
                    let mut files = extract_file_ids(with_files.file_ids).unwrap_or(vec![]);
                    if let Some(embargo_id) = extract_embargo(&self.metadata)? {
                        files.push(embargo_id)
                    }
                    Some(files)
                },
                HyraxModel::Permissions => {
                    let with_perm = AccessTo::deserialize(&self.metadata)?;
                    let access_to_ids = with_perm.access_to.into_iter().map(|member| member.id).collect::<Vec<String>>();
                    Some(access_to_ids)
                },
                _ => None,
            };

            Ok(OrmResource::from_row(self.id.to_string(), self.metadata.to_string(), refs, model))
        } else {
            Err(anyhow!("Not an ORM resource"))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn parses_metadata_json() {
        let json_str = r#"{"doi": [], "label": [], "state": [{"@id": "http://fedora.info/definitions/1/0/access/ObjState#active"}], "title": ["Library catalogue. By B.M. Lelong."], "source": [], "creator": ["California. State Commission of Horticulture."], "keyword": ["Number.", "Brannt", "Culturist.", "National", "Henderson"], "license": [], "subject": [], "abstract": [], "language": [], "depositor": ["admin@example.com"], "publisher": [], "based_near": [], "identifier": ["ark:/13960/t2w37nb3w"], "member_ids": [{"id": "72e654c5-d00e-4d4f-a4d2-7b158a85eb35"}, {"id": "13162ed1-fe7c-4487-b9e5-549433f0f338"}, {"id": "fca73b88-abfc-4321-af1c-dc628b4f1e61"}, {"id": "d1ea6b25-23b3-43c5-ba8c-deceb5ac3118"}], "new_record": [false], "contributor": [], "description": [], "related_url": ["https://archive.org/download/librarycatalogue00calirich/librarycatalogue00calirich_djvu.txt"], "access_right": [], "admin_set_id": [{"id": "5e83a5ef-85b6-4d4b-b20a-b5d4837aa13f"}], "date_created": ["1892"], "rights_notes": [], "thumbnail_id": [{"id": "d1ea6b25-23b3-43c5-ba8c-deceb5ac3118"}], "alternate_ids": [], "date_modified": ["2026-10-16T17:22:00.713+00:00"], "date_uploaded": ["2026-10-16T17:22:00.713+00:00"], "rendering_ids": [], "resource_type": [], "gw_affiliation": [], "arkivo_checksum": [], "rights_statement": [], "alternative_title": [], "representative_id": [{"id": "72e654c5-d00e-4d4f-a4d2-7b158a85eb35"}], "bulkrax_identifier": ["60350ac7-50f8-409e-826a-9e892725299c"], "bibliographic_citation": [], "member_of_collection_ids": []}"#;

        let json_value = serde_json::from_str(json_str).unwrap();
        let work_types = ["ArchivalDocument"];
        let orm_resource_row = OrmResourceRow {
            id: Uuid::new_v4(),
            internal_resource: "ArchivalDocument".to_string(),
            created_at: NaiveDate::from_ymd_opt(2026, 01, 01).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2026, 01, 01).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            metadata: json_value,
            lock_version: 0,
        };
        let orm_resource = orm_resource_row.to_orm_resource(&work_types);
        assert!(orm_resource.is_ok());

    }
}
