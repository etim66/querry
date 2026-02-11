use std::{error::Error, rc::Rc};

use slint::{ComponentHandle, Model, VecModel};
use sqlx::SqlitePool;

use crate::{
    app::handlers::images::load_icon,
    utils::crud::collections::{
        create_collection, delete_collection, get_all_collections, search_collections,
        update_collection_item, CollectionData,
    },
    AppConfig, AppWindow, CollectionItem, SelectedRequestItem,
};

fn to_collection_item(item: CollectionData) -> Option<CollectionItem> {
    let icon_item = load_icon(&item.icon).ok()?;
    Some(CollectionItem {
        id: item.id.into(),
        name: item.name.into(),
        icon: icon_item,
        icon_name: item.icon.into(),
        request_count: item.requests_count,
    })
}

fn to_collection_items(items: Vec<CollectionData>) -> Vec<CollectionItem> {
    items.into_iter().filter_map(to_collection_item).collect()
}

/// Set page to view on app start.
pub async fn check_startup_page(db: &SqlitePool, app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let collection_items = get_all_collections(db).await?;
    let page = if collection_items.is_empty() { 1 } else { 2 };

    let config = app.global::<AppConfig>();
    config.set_page(page);

    Ok(())
}

/// Load all collections on app start.
pub async fn load_collections(db: &SqlitePool, app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let collection_items = get_all_collections(db).await?;
    let collection_data = to_collection_items(collection_items);
    let items_model = Rc::new(VecModel::from(collection_data));

    let config = app.global::<AppConfig>();
    config.set_collection_items(items_model.clone().into());

    Ok(())
}

/// Change page on request.
pub async fn register_page_change(app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    config.on_change_page(move |page| {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();

        cfg.set_page(page);
    });

    Ok(())
}

/// Register collection fetch.
pub async fn register_get_collections(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_get_collections(move || {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let collection_items = match get_all_collections(&db_copy_for_task).await {
                Ok(data) => data,
                Err(_) => [].to_vec(),
            };
            let collection_data = to_collection_items(collection_items);
            let items_model = Rc::new(VecModel::from(collection_data));

            cfg.set_collection_items(items_model.clone().into());
        });
    });

    Ok(())
}

/// Register collection creation.
pub async fn register_create_collection(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_create_collection(move || {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let new_collection =
                match create_collection("New Collection".to_string(), &db_copy_for_task).await {
                    Ok(data) => data,
                    Err(error) => {
                        eprintln!("Error creating collection - {error}");
                        return;
                    }
                };
            let collection_item = match to_collection_item(new_collection) {
                Some(item) => item,
                None => return,
            };

            let mut items: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();
            items.insert(0, collection_item);
            cfg.set_collection_items(Rc::new(VecModel::from(items)).into());
        });
    });
    Ok(())
}

/// Register collection updates.
pub async fn register_update_collection(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_update_collection(move |id, name, icon, index, requests_count| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let new_collection =
                match update_collection_item(&id, &name, &icon, requests_count, &db_copy_for_task)
                    .await
                {
                    Ok(data) => data,
                    Err(error) => {
                        eprintln!("Error updating collection - {error}");
                        return;
                    }
                };
            let icon_item = match load_icon(&new_collection.icon) {
                Ok(data) => data,
                Err(error) => {
                    eprintln!("Error loading image - {error}");
                    return;
                }
            };

            let collection_item = CollectionItem {
                id: new_collection.id.into(),
                name: new_collection.name.into(),
                icon: icon_item.clone(),
                icon_name: new_collection.icon.into(),
                request_count: new_collection.requests_count,
            };

            let mut items: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();
            if let Some(item_ref) = items.get_mut(index as usize) {
                *item_ref = collection_item;
            }
            cfg.set_collection_items(Rc::new(VecModel::from(items)).into());

            // Update the icon on selected requests linked to this collection.
            let mut selected_requests: Vec<SelectedRequestItem> =
                cfg.get_selected_requests().iter().collect();

            for item in selected_requests.iter_mut() {
                if item.collection_id == id {
                    item.collection_icon = icon_item.clone();
                }
            }
            cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
        });
    });
    Ok(())
}

/// Register collection removal.
pub async fn register_remove_collection(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_remove_collection(move |id, index| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            if let Err(error) = delete_collection(&id, &db_copy_for_task).await {
                eprintln!("Error deleting collection - {error}");
                return;
            };
            let mut items: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();
            if items.get_mut(index as usize).is_some() {
                items.remove(index as usize);
            }
            cfg.set_collection_items(Rc::new(VecModel::from(items)).into());

            // Remove selected requests.
            let mut selected_requests: Vec<SelectedRequestItem> =
                cfg.get_selected_requests().iter().collect();

            selected_requests.retain(|item| item.collection_id != id);
            cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
        });
    });
    Ok(())
}

/// Register collection search.
pub async fn register_search_collections(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_search_collection(move |search_string| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let collection_items: Vec<CollectionData> = if search_string.is_empty() {
                match get_all_collections(&db_copy_for_task).await {
                    Ok(data) => data,
                    Err(_) => [].into(),
                }
            } else {
                match search_collections(&db_copy_for_task, &search_string).await {
                    Ok(data) => data,
                    Err(_) => [].into(),
                }
            };

            let collection_data = to_collection_items(collection_items);
            let items_model = Rc::new(VecModel::from(collection_data));

            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            cfg.set_collection_items(items_model.clone().into());
        });
    });

    Ok(())
}
