use slint::{ComponentHandle, Image, VecModel};
use std::{error::Error, path::PathBuf, rc::Rc};

use crate::{utils::get_icon_pack_names, AppConfig, AppWindow, IconsModel};

pub fn load_icon(icon: &str) -> Result<Image, Box<dyn Error>> {
    let path = PathBuf::from(format!("ui/icons_pack/{icon}"));
    Ok(Image::load_from_path(&path)?)
}

pub fn load_icons(app: &AppWindow) -> Result<(), Box<dyn Error>> {
    let mut image_paths: Vec<PathBuf> = Vec::new();

    let icon_items = get_icon_pack_names()?;
    image_paths.extend(
        icon_items
            .iter()
            .map(|icon| PathBuf::from(format!("ui/icons_pack/{icon}"))),
    );

    let mut loaded_icons: Vec<IconsModel> = Vec::new();

    for path in image_paths {
        match Image::load_from_path(&path) {
            Ok(image) => {
                loaded_icons.push(IconsModel {
                    image,
                    name: path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string()
                        .into(),
                });
            }
            Err(e) => {
                eprintln!("Error loading image {}: {}", path.display(), e);
            }
        }
    }

    let items_model = Rc::new(VecModel::from(loaded_icons));

    let config = app.global::<AppConfig>();
    config.set_icons(items_model.clone().into());

    Ok(())
}
