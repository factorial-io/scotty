use base64::prelude::*;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// File bytes with a compact, backwards-compatible JSON representation.
///
/// On the wire this is a base64 string. scottyctl <= 0.3.3 base64-encoded the
/// content itself and stored the resulting ASCII in a `Vec<u8>`, which
/// serde_json wrote out as an array of integers (`[73,68,...]`) — roughly 5.8x
/// the compressed size, so a 12 MB app arrived as a ~70 MB request and tripped
/// `api.create_app_max_size`.
///
/// Deserialization still accepts that array form; because the two encodings are
/// distinguishable on the wire (JSON string vs. JSON array) no client version
/// needs to be negotiated. `double_encoded` records which one arrived so
/// [`FileContent::decode`] can strip the extra base64 layer for old clients.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct FileContent {
    bytes: Vec<u8>,
    double_encoded: bool,
}

impl FileContent {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            double_encoded: false,
        }
    }

    /// The actual file bytes, stripping the extra base64 layer sent by legacy
    /// clients. Still gzip-compressed if the enclosing [`File::compressed`] is set.
    pub fn decode(&self) -> Result<Cow<'_, [u8]>, base64::DecodeError> {
        if self.double_encoded {
            Ok(Cow::Owned(BASE64_STANDARD.decode(&self.bytes)?))
        } else {
            Ok(Cow::Borrowed(&self.bytes))
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl From<Vec<u8>> for FileContent {
    fn from(bytes: Vec<u8>) -> Self {
        Self::new(bytes)
    }
}

impl std::ops::Deref for FileContent {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl AsRef<[u8]> for FileContent {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl std::fmt::Debug for FileContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{} bytes>", self.bytes.len())
    }
}

impl Serialize for FileContent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Always emit the new form. Content read from a legacy client is
        // normalized here so a round-trip can't leave the extra layer behind.
        let bytes = self.decode().unwrap_or(Cow::Borrowed(&self.bytes));
        serializer.serialize_str(&BASE64_STANDARD.encode(bytes))
    }
}

impl<'de> Deserialize<'de> for FileContent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ContentVisitor;

        impl<'de> Visitor<'de> for ContentVisitor {
            type Value = FileContent;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a base64 string or an array of bytes")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                BASE64_STANDARD
                    .decode(v)
                    .map(FileContent::new)
                    .map_err(E::custom)
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(FileContent::new(v.to_vec()))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(byte) = seq.next_element::<u8>()? {
                    bytes.push(byte);
                }
                // Legacy scottyctl: these bytes are base64 ASCII, not file content.
                Ok(FileContent {
                    bytes,
                    double_encoded: true,
                })
            }
        }

        deserializer.deserialize_any(ContentVisitor)
    }
}

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct File {
    pub name: String,
    #[schema(value_type = String, format = "Base64")]
    pub content: FileContent,
    /// Indicates if the content is gzip-compressed
    #[serde(default)]
    pub compressed: bool,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("name", &self.name)
            .field("content", &format!("<{} bytes>", self.content.len()))
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FileList {
    pub files: Vec<File>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_form_round_trips_as_a_base64_string() {
        let file = File {
            name: "compose.yml".to_string(),
            content: b"services:\n".to_vec().into(),
            compressed: false,
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains(r#""content":"c2VydmljZXM6Cg==""#), "{json}");

        let parsed: File = serde_json::from_str(&json).unwrap();
        assert_eq!(&*parsed.content.decode().unwrap(), b"services:\n");
    }

    #[test]
    fn legacy_byte_array_of_base64_is_decoded_once_more() {
        // What scottyctl <= 0.3.3 put on the wire: base64 ASCII as an int array.
        let legacy = serde_json::json!({
            "name": "compose.yml",
            "content": b"c2VydmljZXM6Cg==".to_vec(),
            "compressed": false,
        });

        let parsed: File = serde_json::from_value(legacy).unwrap();
        assert_eq!(&*parsed.content.decode().unwrap(), b"services:\n");

        // Re-serializing normalizes to the new form.
        let json = serde_json::to_string(&parsed).unwrap();
        assert!(json.contains(r#""content":"c2VydmljZXM6Cg==""#), "{json}");
    }

    #[test]
    fn new_form_is_far_smaller_on_the_wire() {
        let content: Vec<u8> = (0..u8::MAX).cycle().take(100_000).collect();
        let file = File {
            name: "blob".to_string(),
            content: content.clone().into(),
            compressed: true,
        };

        let new_form = serde_json::to_string(&file).unwrap().len();
        let legacy_form = serde_json::to_string(&serde_json::json!({
            "name": "blob",
            "content": BASE64_STANDARD.encode(&content).into_bytes(),
            "compressed": true,
        }))
        .unwrap()
        .len();

        assert!(
            new_form * 3 < legacy_form,
            "new {new_form} vs legacy {legacy_form}"
        );
    }
}
