//! The `DISCOVER` screen view's carousel snapshot, in 1.x's shape:
//! `{"location", "carousels": {key: [{"id", "type", "position"}]}}`.

use serde_json::{json, Map, Value};

use super::data_definition::{SegmentEvent, SegmentEventScreenViewed};

/// One Discover carousel as it shows; an item's index is its `position`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct DiscoverCarousel {
    /// The key the place sheet also logs as `carousel` (`featured`, `most_active`, ...).
    pub key: String,
    pub items: Vec<DiscoverCarouselItem>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct DiscoverCarouselItem {
    pub id: String,
    pub world: bool,
}

pub fn discover_snapshot(location: &str, carousels: &[DiscoverCarousel]) -> SegmentEvent {
    let carousels: Map<String, Value> = carousels
        .iter()
        .map(|carousel| {
            let items = carousel
                .items
                .iter()
                .enumerate()
                .map(|(position, item)| {
                    json!({
                        "id": item.id,
                        "type": if item.world { "world" } else { "scene" },
                        "position": position,
                    })
                })
                .collect();
            (carousel.key.clone(), Value::Array(items))
        })
        .collect();
    SegmentEvent::ScreenViewed(SegmentEventScreenViewed {
        screen_name: "DISCOVER".into(),
        extra_properties: Some(json!({"location": location, "carousels": carousels}).to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_each_carousel_with_its_slots() {
        let item = |id: &str, world| DiscoverCarouselItem {
            id: id.into(),
            world,
        };
        let event = discover_snapshot(
            "pre_game",
            &[
                DiscoverCarousel {
                    key: "featured".into(),
                    items: vec![item("a", false), item("b", true)],
                },
                DiscoverCarousel {
                    key: "events".into(),
                    items: vec![item("e", false)],
                },
            ],
        );
        let SegmentEvent::ScreenViewed(screen) = event else {
            panic!("not a screen view");
        };
        assert_eq!(screen.screen_name, "DISCOVER");
        let extra: Value = serde_json::from_str(&screen.extra_properties.unwrap()).unwrap();
        assert_eq!(
            extra,
            json!({
                "location": "pre_game",
                "carousels": {
                    "featured": [
                        {"id": "a", "type": "scene", "position": 0},
                        {"id": "b", "type": "world", "position": 1},
                    ],
                    "events": [{"id": "e", "type": "scene", "position": 0}],
                },
            })
        );
    }
}
