use std::{error::Error, rc::Rc};

use slint::{ComponentHandle, Model, VecModel};
use sqlx::SqlitePool;

use crate::{
    callbacks::images::load_image_item,
    utils::crud::collections::{
        create_collection, delete_collection, get_all_collections, search_collections,
        update_collection_item, CollectionData,
    },
    AppConfig, AppWindow, CollectionItem, SelectedRequestItem,
};

/// Set page to view on app start.
pub async fn check_startup_page(db: &SqlitePool, app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let collection_items = get_all_collections(db).await?;
    let mut page: i32 = 1;

    if !collection_items.is_empty() {
        page = 2;
    }

    let config = app.global::<AppConfig>();
    config.set_page(page);

    Ok(())
}

/// Create load all collections on app start.
pub async fn load_collections(db: &SqlitePool, app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let collection_items = get_all_collections(db).await?;

    let mut collection_data: Vec<CollectionItem> = Vec::new();
    for collection_item in collection_items {
        let icon_item = match load_image_item(&collection_item.icon) {
            Ok(data) => data,
            Err(_) => {
                continue;
            }
        };
        collection_data.push(CollectionItem {
            id: collection_item.id.into(),
            name: collection_item.name.into(),
            icon: icon_item,
            icon_name: collection_item.icon.into(),
            request_count: collection_item.requests_count,
        });
    }
    let items_model = Rc::new(VecModel::from(collection_data));

    let config = app.global::<AppConfig>();
    config.set_collection_items(items_model.clone().into());

    Ok(())
}

/// Change page on ask
pub async fn process_page_change(app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    config.on_change_page(move |page| {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();

        cfg.set_page(page);
    });

    Ok(())
}

/// Get collections
pub async fn process_get_collections(
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
            let mut collection_data: Vec<CollectionItem> = Vec::new();

            for collection_item in collection_items {
                let icon_item = match load_image_item(&collection_item.icon) {
                    Ok(data) => data,
                    Err(_) => {
                        continue;
                    }
                };
                collection_data.push(CollectionItem {
                    id: collection_item.id.into(),
                    name: collection_item.name.into(),
                    icon: icon_item,
                    icon_name: collection_item.icon.into(),
                    request_count: collection_item.requests_count,
                });
            }
            let items_model = Rc::new(VecModel::from(collection_data));

            cfg.set_collection_items(items_model.clone().into());
        });
    });

    Ok(())
}

/// Create a collection
pub async fn process_create_collection(
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
                        eprintln!("Error Creating collection  - {}", error);
                        return;
                    }
                };
            let icon_item = match load_image_item(&new_collection.icon) {
                Ok(data) => data,
                Err(error) => {
                    eprintln!("Error loading image  - {}", error);
                    return;
                }
            };
            let collection_item = CollectionItem {
                id: new_collection.id.into(),
                name: new_collection.name.into(),
                icon: icon_item,
                icon_name: new_collection.icon.into(),
                request_count: new_collection.requests_count,
            };

            let mut items: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();
            items.insert(0, collection_item);
            cfg.set_collection_items(Rc::new(VecModel::from(items)).into());
        });
    });
    Ok(())
}

/// Update a collection
pub async fn process_update_collection(
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
                        eprintln!("Error updating collection  - {}", error);
                        return;
                    }
                };
            let icon_item = match load_image_item(&new_collection.icon) {
                Ok(data) => data,
                Err(error) => {
                    eprintln!("Error loading image  - {}", error);
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
                    item.collection_icon = icon_item;
                    break;
                }
            }
            cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
        });
    });
    Ok(())
}

/// Update a collection
pub async fn process_remove_collection(
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

            match delete_collection(&id, &db_copy_for_task).await {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("Error deleting collection  - {}", error);
                    return;
                }
            };
            let mut items: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();
            if items.get_mut(index as usize).is_some() {
                items.remove(index as usize);
            }
            cfg.set_collection_items(Rc::new(VecModel::from(items)).into());
        });
    });
    Ok(())
}

/// Search collections
pub async fn process_search_collections(
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

            let mut collection_data: Vec<CollectionItem> = Vec::new();

            for collection_item in collection_items {
                let icon_item = match load_image_item(&collection_item.icon) {
                    Ok(data) => data,
                    Err(_) => {
                        continue;
                    }
                };
                collection_data.push(CollectionItem {
                    id: collection_item.id.into(),
                    name: collection_item.name.into(),
                    icon: icon_item,
                    icon_name: collection_item.icon.into(),
                    request_count: collection_item.requests_count,
                });
            }
            let items_model = Rc::new(VecModel::from(collection_data));

            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            cfg.set_collection_items(items_model.clone().into());
        });
    });

    Ok(())
}
