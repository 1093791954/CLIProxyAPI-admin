use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use include_dir::{Dir, include_dir};

static ASSETS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub async fn serve_asset(Path(path): Path<String>) -> Response {
    let asset_path = path.trim_start_matches('/');
    if asset_path.is_empty() {
        return (StatusCode::NOT_FOUND, "asset not found").into_response();
    }

    let Some(file) = ASSETS_DIR.get_file(asset_path) else {
        return (StatusCode::NOT_FOUND, "asset not found").into_response();
    };

    let content_type = mime_guess::from_path(asset_path)
        .first_or_octet_stream()
        .to_string();

    (
        [(header::CONTENT_TYPE, content_type)],
        file.contents().to_vec(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn serves_embedded_style_css() {
        let resp = serve_asset(Path("style.css".to_string())).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(content_type.contains("text/css"));

        let body = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert!(!body.is_empty());
    }

    #[tokio::test]
    async fn returns_404_for_missing_asset() {
        let resp = serve_asset(Path("missing-file.css".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
