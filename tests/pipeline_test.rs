extern crate hyrax_ocfl_pipeline;

use hyrax_ocfl_pipeline::{resources::*, extract::*};
use sqlx::postgres::{Postgres, PgPoolOptions};
use sqlx::pool::Pool;
use sqlx::Error;
use sqlx::Row;
use tokio;
use futures_util::{StreamExt, TryStreamExt};
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, BufWriter};
use serde_json;

async fn init_pool() -> Result<Pool<Postgres>, sqlx::Error> {
    let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://hyrax_user:hyrax_password@localhost/scholarspace_dev").await?;
    Ok(pool)

}


/**
 * TO DO: Implement with smaller test batch
 */
#[tokio::test]
async fn extracts_rows() {
    let pool = init_pool().await.unwrap();
    let work_types = ["AcademicDocument", "ArchivalDocument"];

    let mut stream = sqlx::query_as::<_, OrmResourceRow>("SELECT * FROM orm_resources").fetch(&pool).map_ok(|row| {
       row.to_orm_resource(&work_types)
    });

    /* TO DO: How to refactor this bit to use try_collect()?  */
    let mut results = vec![];
    let mut errors = vec![];

    while let Some(row) = stream.try_next().await.unwrap() {
        if let Ok(row) = row {
            results.push(row);
        } else {
            errors.push(row);
        }
    }

    assert_eq!(errors.len(), 1);

    let mut counter: HashMap<HyraxModel, usize> = HashMap::new();


    for (key, group) in &results.iter().chunk_by(|row| row.model) {
        let group_len = group.collect::<Vec<&OrmResource>>().len();
        counter.entry(key)
                .and_modify(|i| *i += group_len)
                .or_insert(group_len);
    }

    for (model, count) in counter.iter() {
        match model {
            HyraxModel::Embargo => assert_eq!(*count, 8),
            HyraxModel::FileSet => assert_eq!(*count, 799),
            HyraxModel::Permissions => assert_eq!(*count, 934),
            HyraxModel::Work => assert_eq!(*count, 134),
            HyraxModel::FileMetadata => assert_eq!(*count, 788)
        }
    }


    /*
    let file = File::create("/Users/dsmith/Downloads/orm_resource.json").unwrap();
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &results).unwrap();
    writer.flush().unwrap();
    */

}
