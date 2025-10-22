use godot::prelude::*;
use url::Url;

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclParseDeepLink {
    #[var]
    location: Vector2i,

    #[var]
    realm: GString,

    #[var]
    params: Dictionary,
}

#[godot_api]
impl IRefCounted for DclParseDeepLink {
    fn init(_base: Base<RefCounted>) -> Self {
        DclParseDeepLink {
            // Due to Option<Vector2i> isn't supported in godot-rust extension, we workaround it by seeting ::MAX as invalid location
            //  Check is_location_defined
            location: Vector2i::MAX,
            realm: GString::new(),
            params: Dictionary::new(),
        }
    }
}

#[godot_api]
impl DclParseDeepLink {
    #[func]
    pub fn parse_decentraland_link(url_str: GString) -> Gd<DclParseDeepLink> {
        let mut return_object = DclParseDeepLink {
            location: Vector2i::MAX,
            realm: GString::new(),
            params: Dictionary::new(),
        };

        if url_str.is_empty() {
            return Gd::from_object(return_object);
        }

        let url = match Url::parse(url_str.to_string().as_str()) {
            Ok(url) => url,
            Err(err) => {
                godot_error!("Deep link URL unparsed {} - {url_str}", err.to_string());
                return Gd::from_object(return_object);
            }
        };

        // Verify scheme
        if url.scheme() != "decentraland" {
            godot_error!("Invalid scheme: expected 'decentraland' - {url_str}");
            return Gd::from_object(return_object);
        }

        // Parse query parameters
        for (key, value) in url.query_pairs() {
            let _ = return_object
                .params
                .insert(key.to_string().to_variant(), value.to_string().to_variant());

            match key.as_ref() {
                "location" | "position" => {
                    // Parse location as "x,y"
                    let coords: Vec<&str> = value.split(',').collect();
                    if coords.len() == 2 {
                        if let (Ok(x), Ok(y)) = (coords[0].parse::<i32>(), coords[1].parse::<i32>())
                        {
                            return_object.location = Vector2i { x, y };
                        }
                    }
                }
                "realm" => {
                    return_object.realm = value.to_string().into();
                }
                _ => {}
            }
        }

        Gd::from_object(return_object)
    }

    #[func]
    pub fn is_location_defined(&self) -> bool {
        self.location.x < 1000000
    }
}
