use std::sync::Arc;

use axum::Router;
use axum::response::Json;
use axum::routing::get;
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Serialize)]
struct CategoryItem {
    id: i32,
    name: &'static str,
    parent_id: Option<i32>,
}

static CATEGORY_TREE: &[(i32, &str, Option<i32>)] = &[
    // Movies
    (2000, "Movies", None),
    (2030, "Movies/SD", Some(2000)),
    (2040, "Movies/HD", Some(2000)),
    (2045, "Movies/UHD", Some(2000)),
    (2050, "Movies/BluRay", Some(2000)),
    (2060, "Movies/3D", Some(2000)),
    // TV
    (5000, "TV", None),
    (5030, "TV/SD", Some(5000)),
    (5040, "TV/HD", Some(5000)),
    (5045, "TV/UHD", Some(5000)),
    (5070, "TV/Anime", Some(5000)),
    // Audio
    (3000, "Audio", None),
    (3010, "Audio/Lossy", Some(3000)),
    (3040, "Audio/Lossless", Some(3000)),
    (3030, "Audio/Audiobook", Some(3000)),
    // PC
    (4000, "PC", None),
    (4050, "PC/Games", Some(4000)),
    (4010, "PC/Software", Some(4000)),
    (4030, "PC/Mac", Some(4000)),
    // Console
    (1000, "Console", None),
    (1080, "Console/PS", Some(1000)),
    (1010, "Console/Xbox", Some(1000)),
    (1030, "Console/Switch", Some(1000)),
    // Books
    (7000, "Books", None),
    (7010, "Books/Ebook", Some(7000)),
    (7020, "Books/Comics", Some(7000)),
    // XXX
    (6000, "XXX", None),
    // Other
    (8000, "Other", None),
];

async fn get_categories() -> Json<serde_json::Value> {
    let categories: Vec<serde_json::Value> = CATEGORY_TREE
        .iter()
        .map(|(id, name, parent_id)| {
            serde_json::json!({
                "id": id,
                "name": name,
                "parent_id": parent_id,
            })
        })
        .collect();
    Json(serde_json::json!({ "categories": categories }))
}

/// Compute Newznab category from content attributes.
pub fn compute_category(
    content_type: Option<&str>,
    resolution: Option<&str>,
    platform: Option<&str>,
    is_3d: bool,
    is_anime: bool,
    music_format: Option<&str>,
    video_source: Option<&str>,
) -> i32 {
    let ct = match content_type {
        Some(ct) => ct,
        None => return 8000,
    };

    if is_anime {
        return 5070;
    }

    match ct {
        "movie" => {
            if is_3d { return 2060; }
            if video_source == Some("BluRay") && matches!(resolution, Some("2160p" | "1080p")) {
                return 2050;
            }
            match resolution {
                Some("2160p" | "4320p" | "1440p") => 2045,
                Some("1080p" | "720p") => 2040,
                Some("480p" | "360p" | "576p") => 2030,
                _ => 2000,
            }
        }
        "tv_show" => match resolution {
            Some("2160p" | "4320p" | "1440p") => 5045,
            Some("1080p" | "720p") => 5040,
            Some("480p" | "360p" | "576p") => 5030,
            _ => 5000,
        },
        "music" => match music_format {
            Some("lossless") => 3040,
            Some("lossy") => 3010,
            _ => 3000,
        },
        "audiobook" => 3030,
        "game" => match platform {
            Some("PS5" | "PS4" | "PS3" | "PS2") => 1080,
            Some("Xbox Series" | "Xbox One" | "Xbox 360") => 1010,
            Some("Switch") => 1030,
            Some("macOS") => 4030,
            _ => 4050,
        },
        "software" => {
            if platform == Some("macOS") { 4030 } else { 4010 }
        }
        "ebook" => 7010,
        "comic" => 7020,
        "xxx" => 6000,
        _ => 8000,
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/categories", get(get_categories))
}
