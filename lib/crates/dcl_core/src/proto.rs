pub mod social_service {
    include!(concat!(env!("OUT_DIR"), "/decentraland.social_service.rs"));

    #[allow(clippy::all)]
    pub mod v2 {
        include!(concat!(
            env!("OUT_DIR"),
            "/decentraland.social_service.v2.rs"
        ));
    }
}

/// Types the social service protos import; the engine keeps its own copy for components.
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/decentraland.common.rs"));
}
