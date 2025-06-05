use std::{error::Error, fmt};

use sqlx::{query, query_as, FromRow, SqlitePool};
use uuid::Uuid;

use crate::utils::crud::collections::{get_single_collection, update_collection_item};

#[derive(Debug, PartialEq)]
pub enum ProtocolTypes {
    Http,
    Websocket,
    Grpc,
    GraphQL,
}

// Implement the Display trait
impl fmt::Display for ProtocolTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProtocolTypes::Http => write!(f, "HTTP"),
            ProtocolTypes::Websocket => write!(f, "WS"),
            ProtocolTypes::Grpc => write!(f, "GRPC"),
            ProtocolTypes::GraphQL => write!(f, "GQL"),
        }
    }
}

impl ProtocolTypes {
    pub fn from_string(s: &str) -> Option<ProtocolTypes> {
        match s {
            "HTTP" => Some(ProtocolTypes::Http),
            "WS" => Some(ProtocolTypes::Websocket),
            "GRPC" => Some(ProtocolTypes::Grpc),
            "GQL" => Some(ProtocolTypes::GraphQL),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum HTTPMethods {
    Post,
    Get,
    Put,
    Delete,
}

impl fmt::Display for HTTPMethods {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HTTPMethods::Post => write!(f, "POST"),
            HTTPMethods::Get => write!(f, "GET"),
            HTTPMethods::Put => write!(f, "PUT"),
            HTTPMethods::Delete => write!(f, "DEL"),
        }
    }
}

impl HTTPMethods {
    pub fn from_string(s: &str) -> Option<HTTPMethods> {
        match s {
            "POST" => Some(HTTPMethods::Post),
            "GET" => Some(HTTPMethods::Get),
            "PUT" => Some(HTTPMethods::Put),
            "DEL" => Some(HTTPMethods::Delete),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AuthorizationTypes {
    None,
    BearerToken,
}

impl fmt::Display for AuthorizationTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthorizationTypes::None => write!(f, "none"),
            AuthorizationTypes::BearerToken => write!(f, "bearer_token"),
        }
    }
}

impl AuthorizationTypes {
    pub fn from_string(s: &str) -> Option<AuthorizationTypes> {
        match s {
            "none" => Some(AuthorizationTypes::None),
            "bearer_token" => Some(AuthorizationTypes::BearerToken),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
pub struct RequestData {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub protocol: String,
    pub collection_id: String,
    pub http_method: Option<String>,
}

#[derive(Clone, Debug, FromRow)]
pub struct RequestHeaderData {
    pub id: String,
    pub key: String,
    pub value: String,
    pub active: u8,
}

#[derive(Clone, Debug, FromRow)]
pub struct RequestAuthorizationData {
    pub id: String,
    pub auth_type: String,
    pub token: String,
    pub prefix: String,
}

pub async fn get_collection_requests(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<RequestData>, Box<dyn Error>> {
    let requests = query_as("SELECT id, name, url, protocol, http_method, collection_id FROM requestitem WHERE collection_id=$1 ORDER BY created_at DESC").bind(collection_id).fetch_all(pool).await?;

    Ok(requests)
}

pub async fn create_request(
    protocol: ProtocolTypes,
    collection_id: &str,
    pool: &SqlitePool,
) -> Result<RequestData, Box<dyn Error>> {
    let request: RequestData = query_as(
        "INSERT INTO requestitem (id, name, protocol, http_method, collection_id, url) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, name, url, protocol, http_method, collection_id"
    ).bind(Uuid::new_v4().to_string()).bind("New Request").bind(protocol.to_string()).bind(HTTPMethods::Get.to_string()).bind(collection_id).bind("").fetch_one(pool).await?;

    let collection = get_single_collection(collection_id, pool).await?;
    let requests_count = collection.requests_count + 1;
    update_collection_item(
        collection_id,
        &collection.name,
        &collection.icon,
        requests_count,
        pool,
    )
    .await?;

    // Create default authorization item.
    let _auth = create_request_authorization(
        &request.id,
        &AuthorizationTypes::None.to_string(),
        "",
        "",
        pool,
    )
    .await?;

    Ok(request)
}

pub async fn delete_request(request_id: &str, pool: &SqlitePool) -> Result<(), Box<dyn Error>> {
    let request = get_single_request(request_id, pool).await?;

    query("DELETE FROM requestitem WHERE id=$1")
        .bind(request_id)
        .execute(pool)
        .await?;

    let collection = get_single_collection(&request.collection_id, pool).await?;
    let requests_count = collection.requests_count - 1;
    update_collection_item(
        &request.collection_id,
        &collection.name,
        &collection.icon,
        requests_count,
        pool,
    )
    .await?;

    Ok(())
}

pub async fn get_single_request(
    id: &str,
    pool: &SqlitePool,
) -> Result<RequestData, Box<dyn Error>> {
    let request = query_as(
        "SELECT id, name, url, protocol, http_method, collection_id FROM requestitem WHERE id=$1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(request)
}

/// Update a request item.
pub async fn update_request_item(
    id: &str,
    name: &str,
    protocol: ProtocolTypes,
    http_method: HTTPMethods,
    url: &str,
    pool: &SqlitePool,
) -> Result<RequestData, Box<dyn Error>> {
    let command = "UPDATE requestitem SET name=$1, protocol=$2, http_method=$3, url=$4 WHERE id = $5 RETURNING id, name, url, protocol, http_method, collection_id";
    let request: RequestData = query_as(command)
        .bind(name)
        .bind(protocol.to_string())
        .bind(http_method.to_string())
        .bind(url)
        .bind(id)
        .fetch_one(pool)
        .await?;

    Ok(request)
}

pub async fn create_request_header(
    request_id: &str,
    key: &str,
    value: &str,
    pool: &SqlitePool,
) -> Result<RequestHeaderData, Box<dyn Error>> {
    let command = "INSERT INTO requestheader (id, key, value, request_id) VALUES ($1, $2, $3, $4) RETURNING id, key, value, active";
    let header: RequestHeaderData = query_as(command)
        .bind(Uuid::new_v4().to_string())
        .bind(key)
        .bind(value)
        .bind(request_id)
        .fetch_one(pool)
        .await?;

    Ok(header)
}

/// Update a request item.
pub async fn update_request_header(
    id: &str,
    key: &str,
    value: &str,
    active: u8,
    pool: &SqlitePool,
) -> Result<RequestHeaderData, Box<dyn Error>> {
    let command = "UPDATE requestheader SET key=$1, value=$2, active=$3 WHERE id = $4 RETURNING id, key, value, active";
    let header: RequestHeaderData = query_as(command)
        .bind(key)
        .bind(value)
        .bind(active)
        .bind(id)
        .fetch_one(pool)
        .await?;

    Ok(header)
}

pub async fn get_request_headers(
    pool: &SqlitePool,
    request_id: &str,
) -> Result<Vec<RequestHeaderData>, Box<dyn Error>> {
    let headers = query_as("SELECT id, key, value, active FROM requestheader WHERE request_id=$1 ORDER BY created_at ASC").bind(request_id).fetch_all(pool).await?;

    Ok(headers)
}

pub async fn delete_request_header(
    header_id: &str,
    pool: &SqlitePool,
) -> Result<(), Box<dyn Error>> {
    query("DELETE FROM requestheader WHERE id=$1")
        .bind(header_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_request_authorization(
    pool: &SqlitePool,
    request_id: &str,
) -> Result<Vec<RequestAuthorizationData>, Box<dyn Error>> {
    let authorization = query_as(
        "SELECT id, auth_type, token, prefix FROM requestauthorization WHERE request_id=$1",
    )
    .bind(request_id)
    .fetch_all(pool)
    .await?;

    Ok(authorization)
}

pub async fn delete_request_authorization(
    request_id: &str,
    pool: &SqlitePool,
) -> Result<(), Box<dyn Error>> {
    query("DELETE FROM requestauthorization WHERE request_id=$1")
        .bind(request_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn create_request_authorization(
    request_id: &str,
    auth_type: &str,
    token: &str,
    prefix: &str,
    pool: &SqlitePool,
) -> Result<RequestAuthorizationData, Box<dyn Error>> {
    // Delete existing authorization.
    delete_request_authorization(request_id, pool).await?;

    // Add new authorization.
    let command = "INSERT INTO requestauthorization (id, auth_type, token, prefix, request_id) VALUES ($1, $2, $3, $4, $5) RETURNING id, auth_type, token, prefix";
    let header: RequestAuthorizationData = query_as(command)
        .bind(Uuid::new_v4().to_string())
        .bind(auth_type)
        .bind(token)
        .bind(prefix)
        .bind(request_id)
        .fetch_one(pool)
        .await?;

    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_test_db;
    use crate::utils::crud::collections::create_collection;

    #[tokio::test]
    async fn test_create_request() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let _request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant get collections");

        let requests = get_collection_requests(&db.clone(), &collection.id)
            .await
            .unwrap();
        assert!(requests.len() == 1);
        assert!(requests[0].name == "New Request");
        assert!(requests[0].protocol == "HTTP");
        assert!(requests[0].http_method == Some("GET".to_string()));
    }

    #[tokio::test]
    async fn test_get_collection_requests() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let _request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant get collections");

        let requests = get_collection_requests(&db.clone(), &collection.id)
            .await
            .unwrap();
        assert!(requests.len() == 1);
        assert!(requests[0].name == "New Request");
        assert!(requests[0].protocol == "HTTP");
        assert!(requests[0].http_method == Some("GET".to_string()));
    }

    #[tokio::test]
    async fn test_get_single_request() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant get collections");

        let fetched_request = get_single_request(&request.id, &db.clone()).await.unwrap();

        assert!(fetched_request.name == "New Request");
        assert!(fetched_request.protocol == "HTTP");
        assert!(fetched_request.http_method == Some("GET".to_string()));
    }

    #[tokio::test]
    async fn test_delete_request() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant get collections");

        let fetched_request = get_single_request(&request.id, &db.clone()).await.unwrap();

        assert!(fetched_request.name == "New Request");
        assert!(fetched_request.protocol == "HTTP");
        assert!(fetched_request.http_method == Some("GET".to_string()));

        delete_request(&fetched_request.id, &db.clone())
            .await
            .unwrap();
        let fetched_request_new = get_single_request(&fetched_request.id, &db.clone()).await;
        assert!(fetched_request_new.is_err())
    }

    #[tokio::test]
    async fn test_update_single_request() {
        let db = setup_test_db().await.expect("Cant setup db.");
        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant get collections");

        let fetched_request = get_single_request(&request.id, &db.clone()).await.unwrap();

        assert!(fetched_request.name == "New Request");
        assert!(fetched_request.protocol == "HTTP");
        assert!(fetched_request.http_method == Some("GET".to_string()));

        let updated_request = update_request_item(
            &fetched_request.id,
            "Hello Request",
            ProtocolTypes::Http,
            HTTPMethods::Get,
            "https://bbc.co.uk",
            &db.clone(),
        )
        .await
        .unwrap();

        println!("{:?}", updated_request);

        assert!(updated_request.name == "Hello Request".to_string());
        assert!(updated_request.url == Some("https://bbc.co.uk".to_string()))
    }

    #[tokio::test]
    async fn test_create_request_headers() {
        let db = setup_test_db().await.expect("Cant setup db.");

        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant create request");

        let header = create_request_header(&request.id, "bin", "https", &db)
            .await
            .unwrap();
        assert!(header.key == "bin".to_string());
        assert!(header.value == "https".to_string());
    }

    #[tokio::test]
    async fn test_update_request_headers() {
        let db = setup_test_db().await.expect("Cant setup db.");

        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant create request");

        let header = create_request_header(&request.id, "bin", "https", &db)
            .await
            .unwrap();

        let updated_header = match update_request_header(&header.id, "bin", "grpc", 0, &db).await {
            Ok(data) => data,
            Err(error) => {
                eprintln!("{error}");
                assert!(false);
                return;
            }
        };
        assert!(updated_header.key == "bin".to_string());
        assert!(updated_header.value == "grpc".to_string());
    }

    #[tokio::test]
    async fn test_get_request_headers() {
        let db = setup_test_db().await.expect("Cant setup db.");

        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant create request");

        let _header = create_request_header(&request.id, "bin", "https", &db)
            .await
            .unwrap();

        let headers = get_request_headers(&db, &request.id).await.unwrap();
        assert!(headers.len() == 1);
    }

    #[tokio::test]
    async fn test_delete_request_headers() {
        let db = setup_test_db().await.expect("Cant setup db.");

        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant create request");

        let header = create_request_header(&request.id, "bin", "https", &db)
            .await
            .unwrap();

        delete_request_header(&header.id, &db).await.unwrap();

        let headers = get_request_headers(&db, &request.id).await.unwrap();
        assert!(headers.len() == 0);
    }

    #[tokio::test]
    async fn test_create_request_auth() {
        let db = setup_test_db().await.expect("Cant setup db.");

        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant create request");

        let auth = create_request_authorization(
            &request.id,
            &AuthorizationTypes::BearerToken.to_string(),
            "token",
            "bearer",
            &db,
        )
        .await
        .unwrap();

        let auth_items = get_request_authorization(&db, &request.id).await.unwrap();
        assert!(auth_items.len() == 1);
        assert!(auth_items[0].id == auth.id);
    }

    #[tokio::test]
    async fn test_delete_request_auth() {
        // Creating a new auth deletes tje old one.
        let db = setup_test_db().await.expect("Cant setup db.");

        let collection = create_collection("Test collection".to_string(), &db.clone())
            .await
            .unwrap();
        let request = create_request(ProtocolTypes::Http, &collection.id, &db.clone())
            .await
            .expect("Cant create request");

        let _auth = create_request_authorization(
            &request.id,
            &AuthorizationTypes::BearerToken.to_string(),
            "token",
            "bearer",
            &db,
        )
        .await
        .unwrap();

        let auth = create_request_authorization(
            &request.id,
            &AuthorizationTypes::None.to_string(),
            "",
            "",
            &db,
        )
        .await
        .unwrap();

        let auth_items = get_request_authorization(&db, &request.id).await.unwrap();
        assert!(auth_items.len() == 1);
        assert!(auth_items[0].id == auth.id);
    }
}
