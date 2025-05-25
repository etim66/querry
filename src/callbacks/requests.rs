use std::{error::Error, rc::Rc};

use slint::{ComponentHandle, Model, VecModel};
use sqlx::SqlitePool;

use crate::{
    utils::crud::requests::{
        create_request, delete_request, get_collection_requests, update_request_item, HTTPMethods,
        ProtocolTypes,
    },
    AppConfig, AppWindow, CollectionItem, RequestItem, SelectedRequestItem,
};

/// Get requests
pub async fn process_get_requests(db: &SqlitePool, app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_get_requests(move |collection_id| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let request_items =
                match get_collection_requests(&db_copy_for_task, &collection_id).await {
                    Ok(data) => data,
                    Err(_) => [].to_vec(),
                };

            let request_data: Vec<RequestItem> = request_items
                .into_iter()
                .map(|item| RequestItem {
                    id: item.id.into(),
                    name: item.name.into(),
                    url: item.url.unwrap_or("".to_string()).into(),
                    protocol: item.protocol.into(),
                    http_method: item.http_method.unwrap_or("get".to_string()).into(),
                })
                .collect();

            let items_model = Rc::new(VecModel::from(request_data));
            cfg.set_active_collection_requests(items_model.clone().into());
        });
    });

    Ok(())
}

pub async fn process_create_requests(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_create_request_item(move |collection_id, collection_index| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();
        let _ =
            slint::spawn_local(async move {
                let app = weak_app_for_task.upgrade().unwrap();
                let cfg = app.global::<AppConfig>();

                let request_item =
                    match create_request(ProtocolTypes::Http, &collection_id, &db_copy_for_task)
                        .await
                    {
                        Ok(data) => data,
                        Err(_) => {
                            return;
                        }
                    };

                let request_data: RequestItem = RequestItem {
                    id: request_item.id.into(),
                    name: request_item.name.into(),
                    url: request_item.url.unwrap_or("".to_string()).into(),
                    protocol: request_item.protocol.into(),
                    http_method: request_item.http_method.unwrap_or("get".to_string()).into(),
                };

                let mut items: Vec<RequestItem> =
                    cfg.get_active_collection_requests().iter().collect();
                items.insert(0, request_data);
                cfg.set_active_collection_requests(Rc::new(VecModel::from(items)).into());

                // Get collection and increase request count.
                let mut items: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();

                if let Some(item_ref) = items.get_mut(collection_index as usize) {
                    item_ref.request_count += 1;
                }
                cfg.set_collection_items(Rc::new(VecModel::from(items)).into());
            });
    });

    Ok(())
}

pub async fn process_update_request(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_update_request_item(move |request_id, name, protocol, http_method, url, index| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let request_item = match update_request_item(
                &request_id,
                &name,
                ProtocolTypes::from_string(&protocol).unwrap_or(ProtocolTypes::Http),
                HTTPMethods::from_string(&http_method).unwrap_or(HTTPMethods::Get),
                &url,
                &db_copy_for_task,
            )
            .await
            {
                Ok(data) => data,
                Err(_) => {
                    return;
                }
            };

            let request_data: RequestItem = RequestItem {
                id: request_item.id.into(),
                name: request_item.name.into(),
                url: request_item.url.unwrap_or("".to_string()).into(),
                protocol: request_item.protocol.into(),
                http_method: request_item.http_method.unwrap_or("get".to_string()).into(),
            };

            let mut items: Vec<RequestItem> = cfg.get_active_collection_requests().iter().collect();

            if let Some(item_ref) = items.get_mut(index as usize) {
                *item_ref = request_data;
            }
            cfg.set_active_collection_requests(Rc::new(VecModel::from(items)).into());
        });
    });

    Ok(())
}

pub async fn process_delete_request(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_remove_request_item(move |request_id, request_index, collection_index| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            match delete_request(&request_id, &db_copy_for_task).await {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("Error deleting request  - {}", error);
                    return;
                }
            };

            let mut items: Vec<RequestItem> = cfg.get_active_collection_requests().iter().collect();

            if items.get_mut(request_index as usize).is_some() {
                items.remove(request_index as usize);
            }
            cfg.set_active_collection_requests(Rc::new(VecModel::from(items)).into());

            // Get collection and decrease request count.
            let mut collections: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();

            if let Some(item_ref) = collections.get_mut(collection_index as usize) {
                item_ref.request_count -= 1;
            }
            cfg.set_collection_items(Rc::new(VecModel::from(collections)).into());

            // Remove selected requests linked to this request item.
            let mut selected_requests: Vec<SelectedRequestItem> =
                cfg.get_selected_requests().iter().collect();

            selected_requests.retain(|item| item.item.id != request_id);

            cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
        });
    });

    Ok(())
}

/// Handle when a user clicks on a request
pub async fn process_request_selection(app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    config.on_add_selected_request(move |request_index, collection_index| {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();

        // Get collection.
        let collections: Vec<CollectionItem> = cfg.get_collection_items().iter().collect();
        let active_collection = if let Some(collection) = collections.get(collection_index as usize)
        {
            collection
        } else {
            return;
        };

        // Get request
        let active_requests: Vec<RequestItem> =
            cfg.get_active_collection_requests().iter().collect();
        let selected_request = if let Some(request) = active_requests.get(request_index as usize) {
            request
        } else {
            return;
        };

        // Add request to selected requests.
        let mut selected_requests: Vec<SelectedRequestItem> =
            cfg.get_selected_requests().iter().collect();

        let mut request_already_selected = false;
        for request_item in &selected_requests {
            if request_item.item.id == selected_request.id {
                request_already_selected = true;
            }
        }

        if !request_already_selected {
            selected_requests.push(SelectedRequestItem {
                item: selected_request.clone(),
                collection_icon: active_collection.icon.clone(),
                collection_id: active_collection.id.clone(),
            });
            cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
        }
    });

    Ok(())
}

/// Handle remove selected request
pub async fn process_request_remove(app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    config.on_remove_selected_request(move |request_index| {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();

        // Remove selected request from list.
        let mut selected_requests: Vec<SelectedRequestItem> =
            cfg.get_selected_requests().iter().collect();

        if selected_requests.get(request_index as usize).is_some() {
            selected_requests.remove(request_index as usize)
        } else {
            return;
        };

        cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
    });

    Ok(())
}
