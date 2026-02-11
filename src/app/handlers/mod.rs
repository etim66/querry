use std::error::Error;

use sqlx::SqlitePool;

use crate::AppWindow;

pub mod collections;
pub mod images;
pub mod requests;

pub async fn register_all(db: &SqlitePool, app: &AppWindow) -> Result<(), Box<dyn Error>> {
    collections::register_page_change(app).await?;
    collections::register_get_collections(db, app).await?;
    collections::register_create_collection(db, app).await?;
    collections::register_update_collection(db, app).await?;
    collections::register_remove_collection(db, app).await?;
    collections::register_search_collections(db, app).await?;

    requests::register_create_requests(db, app).await?;
    requests::register_get_requests(db, app).await?;
    requests::register_update_request(db, app).await?;
    requests::register_delete_request(db, app).await?;
    requests::register_request_selection(app).await?;
    requests::register_request_remove(app).await?;
    requests::register_mark_selected_request_active(app).await?;

    requests::register_get_request_headers(db, app).await?;
    requests::register_create_request_header(db, app).await?;
    requests::register_update_request_header(db, app).await?;
    requests::register_remove_request_header(db, app).await?;

    requests::register_get_request_authorization(db, app).await?;
    requests::register_update_request_authorization(db, app).await?;

    Ok(())
}
