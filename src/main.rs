// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::{ComponentHandle, PhysicalSize, PlatformError};

use lib::{
    callbacks::{
        collections::{
            check_startup_page, load_collections, process_create_collection,
            process_get_collections, process_page_change, process_remove_collection,
            process_search_collections, process_update_collection,
        },
        images::process_get_images,
        requests::{
            mark_selected_request_active, process_create_requests, process_delete_request,
            process_get_requests, process_request_remove, process_request_selection,
            process_update_request,
        },
    },
    database::get_database,
    AppWindow,
};

#[tokio::main]
async fn main() -> Result<(), PlatformError> {
    let db = get_database().await.expect("Could not connect to DB");

    let app = AppWindow::new()?;

    check_startup_page(&db, &app).await.unwrap();
    load_collections(&db, &app).await.unwrap();
    process_page_change(&app).await.unwrap();
    process_get_collections(&db, &app).await.unwrap();
    process_create_collection(&db, &app).await.unwrap();
    process_get_images(&app).unwrap();
    process_update_collection(&db, &app).await.unwrap();
    process_remove_collection(&db, &app).await.unwrap();
    process_search_collections(&db, &app).await.unwrap();
    process_create_requests(&db, &app).await.unwrap();
    process_get_requests(&db, &app).await.unwrap();
    process_update_request(&db, &app).await.unwrap();
    process_delete_request(&db, &app).await.unwrap();
    process_request_selection(&app).await.unwrap();
    process_request_remove(&app).await.unwrap();
    mark_selected_request_active(&app).await.unwrap();

    let size: PhysicalSize = PhysicalSize::new(1920, 1080);
    app.set_window_height(size.height as f32);
    app.set_window_width(size.width as f32);

    app.run()?;
    Ok(())
}
