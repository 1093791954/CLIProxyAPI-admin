use serde_json::Value;

use crate::models::TokenUsage;

pub fn extract_usage(body: &[u8], content_type: Option<&str>) -> TokenUsage {
    if body.is_empty() {
        return TokenUsage::default();
    }
    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if content_type.contains("text/event-stream") {
        return extract_usage_from_sse(body);
    }
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        return usage_from_json(&value).normalize();
    }
    TokenUsage::default()
}

pub fn extract_model_from_request(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    let value = serde_json::from_slice::<Value>(body).ok()?;
    value
        .get("model")
        .and_then(Value::as_str)
        .map(|v| v.to_string())
}

pub fn extract_model_from_response(body: &[u8], content_type: Option<&str>) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if content_type.contains("text/event-stream") {
        return extract_model_from_sse(body);
    }
    let value = serde_json::from_slice::<Value>(body).ok()?;
    value
        .get("model")
        .and_then(Value::as_str)
        .map(|v| v.to_string())
        .or_else(|| {
            value
                .get("response")
                .and_then(|v| v.get("model"))
                .and_then(Value::as_str)
                .map(|v| v.to_string())
        })
}

fn extract_usage_from_sse(body: &[u8]) -> TokenUsage {
    let text = match std::str::from_utf8(body) {
        Ok(t) => t,
        Err(_) => return TokenUsage::default(),
    };
    let mut usage = TokenUsage::default();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("data:") {
            continue;
        }
        let payload = line.trim_start_matches("data:").trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(payload) {
            let parsed = usage_from_json(&value).normalize();
            if parsed.total_tokens > 0 {
                usage = parsed;
            }
        }
    }
    usage
}

fn extract_model_from_sse(body: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(body).ok()?;
    let mut model = None;
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("data:") {
            continue;
        }
        let payload = line.trim_start_matches("data:").trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(payload) {
            if let Some(v) = value.get("model").and_then(Value::as_str) {
                model = Some(v.to_string());
            }
        }
    }
    model
}

fn usage_from_json(root: &Value) -> TokenUsage {
    let usage = root
        .get("usage")
        .or_else(|| root.get("response").and_then(|v| v.get("usage")));

    let Some(usage) = usage else {
        return TokenUsage::default();
    };

    let input_tokens = first_i64(
        usage.get("prompt_tokens"),
        usage.get("input_tokens"),
        None,
        None,
    );
    let output_tokens = first_i64(
        usage.get("completion_tokens"),
        usage.get("output_tokens"),
        None,
        None,
    );
    let reasoning_tokens = first_i64(
        usage
            .get("completion_tokens_details")
            .and_then(|v| v.get("reasoning_tokens")),
        usage
            .get("output_tokens_details")
            .and_then(|v| v.get("reasoning_tokens")),
        None,
        None,
    );
    let cached_tokens = first_i64(
        usage
            .get("prompt_tokens_details")
            .and_then(|v| v.get("cached_tokens")),
        usage
            .get("input_tokens_details")
            .and_then(|v| v.get("cached_tokens")),
        None,
        None,
    );
    let total_tokens = first_i64(usage.get("total_tokens"), None, None, None);

    TokenUsage {
        input_tokens,
        output_tokens,
        reasoning_tokens,
        cached_tokens,
        total_tokens,
    }
}

fn first_i64(a: Option<&Value>, b: Option<&Value>, c: Option<&Value>, d: Option<&Value>) -> i64 {
    for item in [a, b, c, d] {
        if let Some(v) = item {
            if let Some(n) = v.as_i64() {
                return n;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_usage_json() {
        let body = br#"{
          "id":"chatcmpl-1",
          "model":"gpt-4.1",
          "usage":{"prompt_tokens":11,"completion_tokens":7,"total_tokens":18}
        }"#;
        let usage = extract_usage(body, Some("application/json"));
        assert_eq!(usage.input_tokens, 11);
        assert_eq!(usage.output_tokens, 7);
        assert_eq!(usage.total_tokens, 18);
    }

    #[test]
    fn parse_responses_usage_json() {
        let body = br#"{
          "response":{
            "usage":{
              "input_tokens":10,
              "output_tokens":4,
              "output_tokens_details":{"reasoning_tokens":2},
              "input_tokens_details":{"cached_tokens":1},
              "total_tokens":14
            }
          }
        }"#;
        let usage = extract_usage(body, Some("application/json"));
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 4);
        assert_eq!(usage.reasoning_tokens, 2);
        assert_eq!(usage.cached_tokens, 1);
        assert_eq!(usage.total_tokens, 14);
    }

    #[test]
    fn parse_sse_usage() {
        let body = b"data: {\"id\":\"x\"}\n\ndata: {\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3,\"total_tokens\":5},\"model\":\"m1\"}\n\ndata: [DONE]\n";
        let usage = extract_usage(body, Some("text/event-stream"));
        assert_eq!(usage.total_tokens, 5);
        assert_eq!(
            extract_model_from_response(body, Some("text/event-stream")),
            Some("m1".to_string())
        );
    }
}
