use std::error::Error;

use crate::utils::get_icon_pack_names;
use rand::{rng, seq::IndexedRandom};
use sqlx::{query, query_as, FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Clone, Debug, FromRow)]
pub struct CollectionData {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub requests_count: i32,
}

pub async fn get_all_collections(pool: &SqlitePool) -> Result<Vec<CollectionData>, Box<dyn Error>> {
    let collections = query_as(
        "SELECT 
            id, 
            name, 
            icon, 
            requests_count
        FROM 
            collectionitem
        ORDER BY 
            created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(collections)
}

pub async fn search_collections(
    pool: &SqlitePool,
    search_term: &str,
) -> Result<Vec<CollectionData>, Box<dyn Error>> {
    let collections = query_as(
        "SELECT 
            id, 
            name, 
            icon, 
            requests_count
        FROM 
            collectionitem
        WHERE 
            LOWER(name) LIKE LOWER($1)
        ORDER BY 
            created_at DESC",
    )
    .bind(format!("%{}%", search_term))
    .fetch_all(pool)
    .await?;

    Ok(collections)
}

/// Update a collection item.
pub async fn update_collection_item(
    id: &str,
    name: &str,
    icon: &str,
    requests_count: i32,
    pool: &SqlitePool,
) -> Result<CollectionData, Box<dyn Error>> {
    // Update collection
    let command = "UPDATE collectionitem SET name=$1, icon=$2, requests_count=$3 WHERE id = $4 RETURNING id, name, icon, requests_count";
    let collection: CollectionData = query_as(command)
        .bind(name)
        .bind(icon)
        .bind(requests_count)
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(collection)
}

pub async fn create_collection(
    name: String,
    pool: &SqlitePool,
) -> Result<CollectionData, Box<dyn Error>> {
    let icon_items = get_icon_pack_names()?;
    let mut rng = rng();
    let random_icon = icon_items
        .choose(&mut rng)
        .cloned()
        .unwrap_or_else(|| "1F4A6.svg".to_string());
    let collection: CollectionData = query_as(
        "INSERT INTO collectionitem (id, name, icon) VALUES ($1, $2, $3) RETURNING id, name, icon, requests_count",
    )
        .bind(Uuid::new_v4().to_string())
        .bind(name)
        .bind(random_icon)
        .fetch_one(pool).await?;

    Ok(collection)
}

pub async fn delete_collection(
    collection_id: &str,
    pool: &SqlitePool,
) -> Result<(), Box<dyn Error>> {
    query("DELETE FROM collectionitem WHERE id=$1")
        .bind(collection_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_single_collection(
    id: &str,
    pool: &SqlitePool,
) -> Result<CollectionData, Box<dyn Error>> {
    let collection: CollectionData =
        query_as("SELECT id, name, icon, requests_count FROM collectionitem WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
    Ok(collection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::setup_test_db,
        utils::crud::requests::{create_request, ProtocolTypes},
    };

    #[tokio::test]
    async fn test_create_collection() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collections".to_string(), &db)
            .await
            .expect("Cant get collections");

        assert!(collection.requests_count == 0);
        let existing_collection = get_all_collections(&db)
            .await
            .expect("cant get collections");

        println!(
            "{} - {} - {} - {}",
            collection.name,
            collection.id,
            collection.icon,
            existing_collection.len()
        );
        assert!(existing_collection.len() == 1);

        create_request(ProtocolTypes::Http, &collection.id, &db)
            .await
            .unwrap();
        let single_collection = get_single_collection(&collection.id, &db)
            .await
            .expect("cant get collections");
        assert!(single_collection.requests_count == 1);
    }

    #[tokio::test]
    async fn test_delete_collection() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db)
            .await
            .expect("Cant get collections");

        delete_collection(&collection.id, &db)
            .await
            .expect("Can't delete");

        let existing_collection = get_all_collections(&db)
            .await
            .expect("cant get collections");
        assert!(existing_collection.len() == 0)
    }

    #[tokio::test]
    async fn test_single_collection() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db)
            .await
            .expect("Cant get collections");

        let single_collection = get_single_collection(&collection.id, &db)
            .await
            .expect("cant get collections");
        assert!(single_collection.id == collection.id);
        assert!(single_collection.name == collection.name);
        assert!(!single_collection.icon.is_empty());
    }

    #[tokio::test]
    async fn test_update_collection() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db)
            .await
            .expect("Cant get collections");

        let single_collection = update_collection_item(
            &collection.id,
            "hey",
            "icon.png",
            collection.requests_count,
            &db.clone(),
        )
        .await
        .expect("cant get collections");

        assert!(single_collection.id == collection.id);
        assert!(single_collection.name == "hey".to_string());
        assert!(single_collection.icon == "icon.png".to_string());
    }

    #[tokio::test]
    async fn test_search_collections() {
        let db = setup_test_db().await.expect("Cant setup db.");

        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .expect("Cant get collections");

        let collections = search_collections(&db.clone(), "tion")
            .await
            .expect("cant get collections");

        assert!(collections.len() > 0);
        assert!(collections[0].id == collection.id);
        assert!(collections[0].name == collection.name);
        assert!(collections[0].icon == collection.icon);
    }

    #[tokio::test]
    async fn test_search_collections_2() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .expect("Cant get collections");

        let collections = search_collections(&db.clone(), "TIO")
            .await
            .expect("cant get collections");

        assert!(collections.len() > 0);
        assert!(collections[0].id == collection.id);
        assert!(collections[0].name == collection.name);
        assert!(collections[0].icon == collection.icon);
    }

    #[tokio::test]
    async fn test_search_collections_3() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .expect("Cant get collections");

        let collections = search_collections(&db.clone(), "TES")
            .await
            .expect("cant get collections");

        assert!(collections.len() > 0);
        assert!(collections[0].id == collection.id);
        assert!(collections[0].name == collection.name);
        assert!(collections[0].icon == collection.icon);
    }

    #[tokio::test]
    async fn test_search_collections_4() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let _collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .expect("Cant get collections");

        let collections = search_collections(&db.clone(), "lal")
            .await
            .expect("cant get collections");

        assert!(collections.len() == 0);
    }
}
