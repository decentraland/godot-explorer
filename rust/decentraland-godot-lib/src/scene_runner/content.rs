use godot::prelude::*;
use std::collections::HashMap;

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct ContentMapping {
    #[base]
    base: Base<Node>,
    base_url: GodotString,
    content_mapping: HashMap<GodotString, GodotString>,
}

#[godot_api]
impl ContentMapping {
    #[func]
    fn set_content_mapping(&mut self, content_mapping: Dictionary) {
        self.content_mapping.clear();
        self.content_mapping.reserve(content_mapping.len());

        for (file, hash) in content_mapping.iter_shared() {
            self.content_mapping
                .insert(file.to_string().into(), hash.to_string().into());
        }
    }

    #[func]
    fn set_base_url(&mut self, base_url: GodotString) {
        self.base_url = base_url;
    }

    #[func]
    fn get_base_url(&self) -> GodotString {
        self.base_url.clone()
    }

    #[func]
    fn get_content_hash(&self, file: GodotString) -> GodotString {
        self.content_mapping
            .get(&file)
            .unwrap_or(&GodotString::from(""))
            .clone()
    }

    #[func]
    fn get_mappings(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        for (file, hash) in self.content_mapping.iter() {
            dict.insert(file.clone(), hash.clone());
        }
        dict
    }
}

#[godot_api]
impl NodeVirtual for ContentMapping {
    fn init(base: Base<Node>) -> Self {
        ContentMapping {
            base,
            base_url: GodotString::from(""),
            content_mapping: HashMap::new(),
        }
    }
}
