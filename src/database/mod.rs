use std::error::Error;

use crate::utils::sys_dir::get_db_path;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub async fn get_database() -> Result<SqlitePool, Box<dyn Error>> {
    let db_path = get_db_path(Some(false))?;
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_path)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Setup a clean database for tests.
pub async fn setup_test_db() -> Result<SqlitePool, Box<dyn Error>> {
    let db_path = get_db_path(Some(true))?;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_path)
        .await?;

    // Drop all tables to start fresh
    sqlx::query(
        r#"
        DROP TABLE IF EXISTS collectionitem;
        DROP TABLE IF EXISTS collectionheader;
        DROP TABLE IF EXISTS requestitem;
        DROP TABLE IF EXISTS _sqlx_migrations;  
        DROP TABLE IF EXISTS requestheader;
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
