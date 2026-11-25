use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::harness::Image;

pub(super) fn control(id: &str, subtype: &str, fields: Value) -> Value {
    let mut request = json!({ "subtype": subtype });
    if let (Some(request), Value::Object(fields)) = (request.as_object_mut(), fields) {
        request.extend(fields);
    }
    json!({ "type": "control_request", "request_id": id, "request": request })
}

pub(super) fn control_success(id: &str, response: Value) -> Value {
    json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": id, "response": response },
    })
}

pub(super) fn control_error(id: &str, error: &str) -> Value {
    json!({
        "type": "control_response",
        "response": { "subtype": "error", "request_id": id, "error": error },
    })
}

pub(super) fn user(uuid: &str, text: &str, images: &[Image]) -> Value {
    let content = match images {
        [] => json!(text),
        _ => {
            let text = (!text.is_empty()).then(|| json!({ "type": "text", "text": text }));
            let images = images.iter().map(|image| {
                json!({
                    "type": "image",
                    "source": { "type": "base64", "media_type": image.mime_type, "data": image.data },
                })
            });
            Value::Array(text.into_iter().chain(images).collect())
        }
    };
    json!({
        "type": "user",
        "uuid": uuid,
        "message": { "role": "user", "content": content },
        "parent_tool_use_id": null,
    })
}

pub(super) fn uuid(count: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos() as u64);
    let bits = (u128::from(nanos) << 64) | u128::from(count);
    let bits = (bits & !(0xf << 76) & !(0x3 << 62)) | (0x4 << 76) | (0x2 << 62);
    let hex = format!("{bits:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}
