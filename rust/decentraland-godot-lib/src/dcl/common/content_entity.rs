use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TypedIpfsRef {
    pub file: String,
    pub hash: String,
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct EntityDefinitionJson {
    pub id: Option<String>,
    pub pointers: Vec<String>,
    pub content: Vec<TypedIpfsRef>,
    pub metadata: Option<serde_json::Value>,
}
