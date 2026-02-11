use std::{error::Error, io};

use slint::{ComponentHandle, PhysicalSize};
use sqlx::SqlitePool;

use crate::{app::handlers, database::get_database, AppWindow};

fn platform_error_to_boxed(err: slint::PlatformError) -> Box<dyn Error> {
    Box::new(io::Error::other(err.to_string()))
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let db = get_database().await?;
    let app = AppWindow::new().map_err(platform_error_to_boxed)?;

    bootstrap(&db, &app).await?;
    handlers::register_all(&db, &app).await?;

    app.run().map_err(platform_error_to_boxed)?;
    Ok(())
}

pub async fn bootstrap(db: &SqlitePool, app: &AppWindow) -> Result<(), Box<dyn Error>> {
    handlers::collections::check_startup_page(db, app).await?;
    handlers::collections::load_collections(db, app).await?;
    handlers::images::load_icons(app)?;

    let size: PhysicalSize = PhysicalSize::new(1920, 1080);
    app.set_window_height(size.height as f32);
    app.set_window_width(size.width as f32);

    Ok(())
}
