//! The `multipart/form-data` body the catalyst's `POST /entities` takes.

pub struct Multipart {
    pub content_type: String,
    pub body: Vec<u8>,
}

/// `fields` are text parts; `file` is the entity file, named by its own id and sent without a
/// filename or content type, as the catalyst expects it.
pub fn build(fields: &[(String, String)], file_name: &str, file: &[u8]) -> Multipart {
    let boundary = format!("dcl{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{file_name}\"\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Multipart {
        content_type: format!("multipart/form-data; boundary={boundary}"),
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_text_parts_then_the_file_between_boundaries() {
        let part = build(&[("entityId".into(), "bafy".into())], "bafy", b"{}");
        let boundary = part.content_type.split("boundary=").nth(1).unwrap();
        let body = String::from_utf8(part.body).unwrap();
        assert_eq!(
            body,
            format!(
                "--{b}\r\nContent-Disposition: form-data; name=\"entityId\"\r\n\r\nbafy\r\n\
                 --{b}\r\nContent-Disposition: form-data; name=\"bafy\"\r\n\r\n{{}}\r\n--{b}--\r\n",
                b = boundary
            )
        );
    }
}
