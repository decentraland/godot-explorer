use rand::{rngs::StdRng, SeedableRng};

use super::*;
use crate::app::{AvatarState, ColorTarget};

const BASE: &str = "urn:decentraland:off-chain:base-avatars:";

fn item(id: &str, category: &str, shapes: &[&str], rarity: &str) -> CatalogItem {
    CatalogItem {
        urn: if id.starts_with("urn:") {
            id.to_string()
        } else {
            format!("{BASE}{id}")
        },
        name: id.to_string(),
        category: category.to_string(),
        thumbnail: "t".to_string(),
        rarity: rarity.to_string(),
        body_shapes: shapes.iter().map(|s| s.to_string()).collect(),
        collectible: rarity != "base",
    }
}

fn catalog() -> Catalog {
    Catalog::new(vec![
        item("eyes_00", "eyes", &[MALE], "base"),
        item("f_eyes_00", "eyes", &[FEMALE], "base"),
        item("eyebrows_00", "eyebrows", &[MALE], "base"),
        item("f_eyebrows_00", "eyebrows", &[FEMALE], "base"),
        item("mouth_00", "mouth", &[MALE], "base"),
        item("f_mouth_00", "mouth", &[FEMALE], "base"),
        item("green_hoodie", "upper_body", &[MALE, FEMALE], "base"),
        item("f_sweater", "upper_body", &[FEMALE], "base"),
        item("beard", "facial_hair", &[MALE], "base"),
        item("hair_a", "hair", &[], "base"),
        item(
            "urn:decentraland:matic:collections-v2:0xab:0",
            "hat",
            &[],
            "epic",
        ),
    ])
}

fn avatar(shape: &str, wearables: &[&str]) -> AvatarState {
    AvatarState {
        body_shape: shape.to_string(),
        wearables: wearables.iter().map(|id| format!("{BASE}{id}")).collect(),
        emotes: vec![String::new(); EMOTE_SLOTS],
        skin: "#ffe4c6".into(),
        hair: "#1c1c1c".into(),
        eyes: "#2b2b2b".into(),
    }
}

#[test]
fn equip_replaces_the_item_of_the_same_category() {
    let start = avatar(FEMALE, &["f_sweater", "hair_a"]);
    let next = equip(&start, &catalog(), &format!("{BASE}green_hoodie")).unwrap();
    assert_eq!(
        next.wearables,
        vec![format!("{BASE}hair_a"), format!("{BASE}green_hoodie")]
    );
    assert_eq!(start.wearables.len(), 2, "the input is untouched");
}

#[test]
fn equip_is_case_insensitive_and_does_not_duplicate() {
    let start = avatar(MALE, &["green_hoodie"]);
    let next = equip(&start, &catalog(), &format!("{BASE}GREEN_HOODIE")).unwrap();
    assert_eq!(next.wearables.len(), 1);
}

#[test]
fn equip_rejects_unknown_items_and_the_wrong_body_shape() {
    let start = avatar(MALE, &[]);
    assert!(matches!(
        equip(&start, &catalog(), "urn:nope"),
        Err(AvatarError::UnknownItem(_))
    ));
    assert_eq!(
        equip(&start, &catalog(), &format!("{BASE}f_sweater")),
        Err(AvatarError::WrongBodyShape(format!("{BASE}f_sweater")))
    );
}

#[test]
fn unequip_removes_optional_items_and_keeps_required_ones() {
    let start = avatar(MALE, &["green_hoodie", "eyes_00"]);
    let next = unequip(&start, &catalog(), &format!("{BASE}green_hoodie"));
    assert_eq!(next.wearables, vec![format!("{BASE}eyes_00")]);
    let same = unequip(&next, &catalog(), &format!("{BASE}eyes_00"));
    assert_eq!(same, next);
}

#[test]
fn body_shape_drops_misfits_and_refills_required_categories() {
    let start = avatar(
        FEMALE,
        &[
            "f_eyes_00",
            "f_eyebrows_00",
            "f_mouth_00",
            "f_sweater",
            "hair_a",
        ],
    );
    let next = set_body_shape(&start, &catalog(), MALE).unwrap();
    assert_eq!(next.body_shape, MALE);
    assert_eq!(
        next.wearables,
        vec![
            format!("{BASE}hair_a"),
            format!("{BASE}eyes_00"),
            format!("{BASE}eyebrows_00"),
            format!("{BASE}mouth_00"),
        ]
    );
}

#[test]
fn body_shape_must_be_one_of_the_two() {
    assert!(matches!(
        set_body_shape(&avatar(MALE, &[]), &catalog(), "urn:other"),
        Err(AvatarError::InvalidBodyShape(_))
    ));
}

#[test]
fn colors_are_validated_and_lowercased() {
    let start = avatar(MALE, &[]);
    assert_eq!(
        set_color(&start, ColorTarget::Hair, "#AABBCC")
            .unwrap()
            .hair,
        "#aabbcc"
    );
    for bad in ["aabbcc", "#abc", "#gggggg", ""] {
        assert!(set_color(&start, ColorTarget::Skin, bad).is_err(), "{bad}");
    }
}

#[test]
fn emote_slots_are_bounded() {
    let start = avatar(MALE, &[]);
    assert_eq!(set_emote_slot(&start, 3, "wave").unwrap().emotes[3], "wave");
    assert_eq!(
        set_emote_slot(&start, 10, "wave"),
        Err(AvatarError::SlotOutOfRange(10))
    );
}

#[test]
fn emote_slots_grow_a_short_list() {
    let mut start = avatar(MALE, &[]);
    start.emotes = vec![];
    let next = set_emote_slot(&start, 9, "dab").unwrap();
    assert_eq!(next.emotes.len(), EMOTE_SLOTS);
    assert_eq!(next.emotes[9], "dab");
}

#[test]
fn randomize_wears_every_required_category_once_and_only_base_items() {
    let start = avatar(MALE, &[]);
    for seed in 0..20 {
        let next = randomize(&start, &catalog(), &mut StdRng::seed_from_u64(seed)).unwrap();
        for category in ["eyes", "eyebrows", "mouth"] {
            let worn = next
                .wearables
                .iter()
                .filter(|urn| catalog().find(urn).unwrap().category == category)
                .count();
            assert_eq!(worn, 1, "seed {seed} {category}");
        }
        assert!(next
            .wearables
            .iter()
            .all(|urn| catalog().find(urn).unwrap().rarity == "base"));
        assert!(next
            .wearables
            .iter()
            .all(|urn| catalog().find(urn).unwrap().fits(&next.body_shape)));
        assert!(palette(ColorTarget::Hair).contains(&next.hair.as_str()));
    }
}

#[test]
fn randomize_refuses_an_empty_catalog() {
    assert_eq!(
        randomize(
            &avatar(MALE, &[]),
            &Catalog::default(),
            &mut StdRng::seed_from_u64(1)
        ),
        Err(AvatarError::CatalogNotLoaded)
    );
}

#[test]
fn randomize_is_deterministic_for_a_seed_and_keeps_the_emotes() {
    let mut start = avatar(MALE, &[]);
    start.emotes[0] = "wave".into();
    let a = randomize(&start, &catalog(), &mut StdRng::seed_from_u64(7)).unwrap();
    let b = randomize(&start, &catalog(), &mut StdRng::seed_from_u64(7)).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.emotes[0], "wave");
}

#[test]
fn parses_the_lambda_payload() {
    let payload = r#"{"wearables":[
        {"id":"urn:decentraland:off-chain:base-avatars:f_sweater","name":"Sweater",
         "i18n":[{"code":"en","text":"Cozy sweater"},{"code":"es","text":"Suéter"}],
         "thumbnail":"https://x/t.png","rarity":"mystery",
         "data":{"category":"upper_body","representations":[
            {"bodyShapes":["urn:decentraland:off-chain:base-avatars:BaseFemale"]},
            {"bodyShapes":["urn:decentraland:off-chain:base-avatars:BaseFemale"]}]}},
        {"id":"urn:decentraland:matic:collections-v2:0xab:0","name":"Hat","thumbnail":"t",
         "rarity":"epic","data":{"category":"hat"}},
        {"id":"no-category","thumbnail":"t","data":{}},
        {"name":"no id","thumbnail":"t","data":{"category":"hat"}}
    ]}"#;
    let items = parse_wearables(payload);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "Cozy sweater");
    assert_eq!(items[0].rarity, "base", "an unknown rarity reads as base");
    assert_eq!(items[0].body_shapes, vec![FEMALE.to_string()]);
    assert!(!items[0].collectible);
    assert_eq!(items[1].rarity, "epic");
    assert!(items[1].collectible);
    assert!(parse_wearables("not json").is_empty());
    assert!(parse_wearables("{}").is_empty());
}

#[test]
fn the_base_emotes_are_the_21_the_client_ships() {
    let emotes = base_emotes();
    assert_eq!(emotes.len(), 21);
    assert_eq!(emotes[1].urn, "urn:decentraland:off-chain:base-emotes:wave");
    assert!(is_base_wearable(
        "URN:DECENTRALAND:OFF-CHAIN:BASE-AVATARS:eyes_00"
    ));
}

struct Reply(Result<(u16, &'static str), &'static str>);

#[async_trait::async_trait]
impl crate::profile::Transport for Reply {
    async fn send(
        &self,
        request: crate::profile::HttpRequest,
    ) -> Result<crate::profile::HttpReply, String> {
        assert!(request.url.ends_with(
            "collections/wearables?collectionId=urn:decentraland:off-chain:base-avatars"
        ));
        self.0
            .map(|(status, body)| crate::profile::HttpReply {
                status,
                body: body.to_string(),
            })
            .map_err(str::to_string)
    }
}

fn run<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn the_base_catalog_is_read_from_the_lambdas() {
    let endpoints = crate::profile::Endpoints::new("https://peer/lambdas", "https://peer/content");
    let body = r#"{"wearables":[{"id":"urn:decentraland:off-chain:base-avatars:f_sweater",
        "thumbnail":"t","data":{"category":"upper_body"}}]}"#;
    let items = run(base_catalog(&Reply(Ok((200, body))), &endpoints)).unwrap();
    assert_eq!(items.len(), 1);

    assert_eq!(
        run(base_catalog(&Reply(Ok((503, ""))), &endpoints)),
        Err(CatalogError::Http(503))
    );
    assert_eq!(
        run(base_catalog(&Reply(Ok((200, "{}"))), &endpoints)),
        Err(CatalogError::Empty)
    );
    assert!(matches!(
        run(base_catalog(&Reply(Err("offline")), &endpoints)),
        Err(CatalogError::Network(_))
    ));
}
