use std::{error::Error, rc::Rc};

use slint::{ComponentHandle, Model, VecModel};
use sqlx::SqlitePool;

use crate::{
    utils::crud::requests::{
        create_request, create_request_authorization, create_request_header, delete_request,
        delete_request_header, get_collection_requests, get_request_authorization,
        get_request_headers, update_request_header, update_request_item, AuthorizationTypes,
        HTTPMethods, ProtocolTypes, RequestAuthorizationData,
    },
    AppConfig, AppWindow, AuthorizationItem, CollectionItem, HeaderItem, RequestItem,
    SelectedRequestItem,
};

impl From<RequestAuthorizationData> for AuthorizationItem {
    fn from(item: RequestAuthorizationData) -> Self {
        AuthorizationItem {
            id: item.id.into(),
            auth_type: item.auth_type.into(),
            token: item.token.into(),
            prefix: item.prefix.into(),
        }
    }
}

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
    config.on_update_request_item(move |request_id, name, protocol, http_method, url| {
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

            for item in items.iter_mut().enumerate() {
                if item.1.id == request_id {
                    *item.1 = request_data.clone();
                    break;
                }
            }
            cfg.set_active_collection_requests(Rc::new(VecModel::from(items)).into());

            // Update item in selected requests.
            let mut selected_requests: Vec<SelectedRequestItem> =
                cfg.get_selected_requests().iter().collect();

            for item in selected_requests.iter_mut() {
                if item.item.id == request_id {
                    item.item = request_data;
                    break;
                }
            }

            cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
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

        let selected_request = SelectedRequestItem {
            item: selected_request.clone(),
            collection_icon: active_collection.icon.clone(),
            collection_id: active_collection.id.clone(),
            collection_index,
        };

        if !request_already_selected {
            selected_requests.push(selected_request.clone());
            cfg.set_selected_requests(Rc::new(VecModel::from(selected_requests)).into());
        }

        // Set selected request as active.
        cfg.set_active_selected_request(selected_request);
        cfg.set_show_request_section(true);
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

/// Handle setting a selected request as active.
pub async fn mark_selected_request_active(app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    config.on_mark_request_active(move |request_index| {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();

        // Remove selected request from list.
        let selected_requests: Vec<SelectedRequestItem> =
            cfg.get_selected_requests().iter().collect();

        let selected_request = if let Some(request) = selected_requests.get(request_index as usize)
        {
            request.clone()
        } else {
            return;
        };

        cfg.set_active_selected_request(selected_request);
    });

    Ok(())
}

/// Get request headers.
pub async fn process_get_request_headers(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_get_request_headers(move |request_id| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let headers = match get_request_headers(&db_copy_for_task, &request_id).await {
                Ok(data) => data,
                Err(_) => [].to_vec(),
            };

            let new_headers: Vec<HeaderItem> = headers
                .into_iter()
                .map(|x| HeaderItem {
                    active: x.active == 1,
                    id: x.id.into(),
                    key: x.key.into(),
                    value: x.value.into(),
                })
                .collect();

            cfg.set_request_item_headers(Rc::new(VecModel::from(new_headers)).into());
        });
    });

    Ok(())
}

/// Create request headers.
pub async fn process_create_request_header(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_create_request_header(move |request_id| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let header = match create_request_header(&request_id, "", "", &db_copy_for_task).await {
                Ok(data) => data,
                Err(_) => return,
            };

            let mut headers: Vec<HeaderItem> = cfg.get_request_item_headers().iter().collect();
            headers.push(HeaderItem {
                active: header.active == 1,
                id: header.id.into(),
                key: header.key.into(),
                value: header.value.into(),
            });

            cfg.set_request_item_headers(Rc::new(VecModel::from(headers)).into());
        });
    });

    Ok(())
}

/// Update request headers.
pub async fn process_update_request_header(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_update_request_header(move |header_id, key, value, active| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let mut processed_active = 0;
            if active {
                processed_active = 1;
            }

            let header = match update_request_header(
                &header_id,
                &key,
                &value,
                processed_active,
                &db_copy_for_task,
            )
            .await
            {
                Ok(data) => data,
                Err(_) => return,
            };

            let mut headers: Vec<HeaderItem> = cfg.get_request_item_headers().iter().collect();
            if let Some(index) = headers.iter().position(|item| item.id == header.id) {
                let _ = std::mem::replace(
                    &mut headers[index],
                    HeaderItem {
                        active: header.active == 1,
                        id: header.id.into(),
                        key: header.key.into(),
                        value: header.value.into(),
                    },
                );
            }

            cfg.set_request_item_headers(Rc::new(VecModel::from(headers)).into());
        });
    });

    Ok(())
}

/// Remove request headers.
pub async fn process_remove_request_header(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_remove_request_header(move |header_id| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            if (delete_request_header(&header_id, &db_copy_for_task).await).is_ok() {};

            let mut headers: Vec<HeaderItem> = cfg.get_request_item_headers().iter().collect();
            headers.retain(|item| item.id != header_id);
            cfg.set_request_item_headers(Rc::new(VecModel::from(headers)).into());
        });
    });

    Ok(())
}

/// Get request auth.
pub async fn process_get_request_authorization(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_get_request_auth(move |request_id| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let authorization =
                match get_request_authorization(&db_copy_for_task, &request_id).await {
                    Ok(data) => data,
                    Err(_) => [].to_vec(),
                };

            let new_authorization: Vec<AuthorizationItem> = authorization
                .into_iter()
                .map(AuthorizationItem::from)
                .collect();

            cfg.set_request_auth_items(Rc::new(VecModel::from(new_authorization)).into());
        });
    });

    Ok(())
}

/// Get request auth.
pub async fn process_update_request_authorization(
    db: &SqlitePool,
    app: &AppWindow,
) -> Result<(), Box<dyn Error>> {
    let config = app.global::<AppConfig>();
    let weak_app = app.as_weak();

    let db_copy = db.clone();
    config.on_update_request_auth(move |request_id, auth_type, token, prefix| {
        let weak_app_for_task = weak_app.clone();
        let db_copy_for_task = db_copy.clone();

        let _ = slint::spawn_local(async move {
            let app = weak_app_for_task.upgrade().unwrap();
            let cfg = app.global::<AppConfig>();

            let validated_auth_type = AuthorizationTypes::from_string(&auth_type)
                .unwrap_or(AuthorizationTypes::None)
                .to_string();
            let authorization = match create_request_authorization(
                &request_id,
                &validated_auth_type,
                &token,
                &prefix,
                &db_copy_for_task,
            )
            .await
            {
                Ok(data) => [data].to_vec(),
                Err(_) => [].to_vec(),
            };

            let new_authorization: Vec<AuthorizationItem> = authorization
                .into_iter()
                .map(AuthorizationItem::from)
                .collect();

            cfg.set_request_auth_items(Rc::new(VecModel::from(new_authorization)).into());
        });
    });

    Ok(())
}
