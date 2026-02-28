use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::state::AppState;

const WWW_AUTH_HEADER_VALUE: &str = "Basic realm=\"CLIProxyAPI Admin\", charset=\"UTF-8\"";

pub async fn require_basic_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let is_authorized = extract_basic_credentials(request.headers().get(header::AUTHORIZATION))
        .map(|(username, password)| {
            username == state.config.admin_username && password == state.config.admin_password
        })
        .unwrap_or(false);

    if is_authorized {
        next.run(request).await
    } else {
        unauthorized_response()
    }
}

fn extract_basic_credentials(auth_header: Option<&HeaderValue>) -> Option<(String, String)> {
    let header_value = auth_header?.to_str().ok()?;
    let (scheme, encoded) = header_value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }

    let decoded = STANDARD.decode(encoded).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (username, password) = decoded.split_once(':')?;
    Some((username.to_string(), password.to_string()))
}

fn unauthorized_response() -> Response {
    let mut response = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static(WWW_AUTH_HEADER_VALUE),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::extract_basic_credentials;
    use axum::http::{HeaderValue, header};
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    #[test]
    fn parses_basic_credentials() {
        let encoded = STANDARD.encode("adminops:S7rong!Token#Keeper2026");
        let header = HeaderValue::from_str(&format!("Basic {encoded}")).expect("valid header");
        let result = extract_basic_credentials(Some(&header)).expect("should parse");
        assert_eq!(result.0, "adminops");
        assert_eq!(result.1, "S7rong!Token#Keeper2026");
    }

    #[test]
    fn rejects_non_basic_scheme() {
        let header = HeaderValue::from_static("Bearer abc");
        let result = extract_basic_credentials(Some(&header));
        assert!(result.is_none());
    }

    #[test]
    fn rejects_invalid_base64_payload() {
        let header = HeaderValue::from_static("Basic not_base64");
        let result = extract_basic_credentials(Some(&header));
        assert!(result.is_none());
    }

    #[test]
    fn rejects_missing_password_separator() {
        let encoded = STANDARD.encode("adminops-no-colon");
        let header = HeaderValue::from_str(&format!("Basic {encoded}")).expect("valid header");
        let result = extract_basic_credentials(Some(&header));
        assert!(result.is_none());
    }

    #[test]
    fn header_constant_is_valid() {
        HeaderValue::from_static("Basic realm=\"CLIProxyAPI Admin\", charset=\"UTF-8\"");
        assert_eq!(header::WWW_AUTHENTICATE.as_str(), "www-authenticate");
    }
}
