/// Parses an artwork type string ("grid", "hero", etc.) into the crate enum.
pub(crate) fn parse_artwork_type(s: &str) -> Option<capydeploy_steam::ArtworkType> {
    match s {
        "grid" => Some(capydeploy_steam::ArtworkType::Grid),
        "hero" => Some(capydeploy_steam::ArtworkType::Hero),
        "logo" => Some(capydeploy_steam::ArtworkType::Logo),
        "icon" => Some(capydeploy_steam::ArtworkType::Icon),
        "portrait" | "banner" => Some(capydeploy_steam::ArtworkType::Portrait),
        _ => None,
    }
}

/// Extracts a file extension from an image MIME type.
pub(crate) fn ext_from_content_type(ct: &str) -> &str {
    match ct {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        _ => "png",
    }
}
