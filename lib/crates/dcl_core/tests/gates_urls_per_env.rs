//! The URL record is complete in every environment. Its own test binary: the environment is
//! process-wide, and the unit tests of the crate read it concurrently.

use dcl_core::{
    env::{self, DclEnvironment},
    gates::urls::{self, Environment},
};

#[test]
fn every_field_is_a_url_in_every_environment() {
    for (env, suffix, expected) in [
        (DclEnvironment::Org, "org", Environment::Org),
        (DclEnvironment::Zone, "zone", Environment::Zone),
        (DclEnvironment::Today, "today", Environment::Today),
    ] {
        env::set_environment(env);
        assert_eq!(urls::environment(), expected);

        let all = urls::build();
        let fields = [
            &all.host,
            &all.help,
            &all.privacy_policy,
            &all.terms_of_service,
            &all.content_policy,
            &all.places_api,
            &all.places,
            &all.events_api,
            &all.jump_events,
            &all.main_realm,
            &all.worlds_content_server,
            &all.peer_base,
            &all.peer_content,
            &all.peer_lambdas,
            &all.mobile_bff,
            &all.destinations_api,
            &all.mobile_events_api,
            &all.account_deletion,
            &all.notifications_api,
            &all.credits_server,
            &all.social_service,
            &all.marketplace,
            &all.marketplace_api,
            &all.marketplace_catalog_api,
            &all.marketplace_claim_name,
            &all.auth_frontend,
            &all.auth_mobile_frontend,
            &all.auth_api_base,
            &all.auth_api_requests,
            &all.mobile_open,
        ];
        for field in fields {
            assert!(
                field.starts_with("https://")
                    || field.starts_with("http://")
                    || field.starts_with("wss://"),
                "{suffix}: not a URL: {field:?}"
            );
        }
        // Services that follow the environment name it (the dev server and the fixed hosts do not).
        for field in [
            &all.host,
            &all.mobile_bff,
            &all.notifications_api,
            &all.credits_server,
        ] {
            assert!(field.contains(&format!(".{suffix}")), "{suffix}: {field}");
        }
        assert!(all.dev_realms.iter().all(|realm| !realm.is_empty()));
        assert!(urls::marketplace_browse("wearables").starts_with(&all.marketplace));
        assert!(urls::jump_position_link("1,2").starts_with(&all.host));
    }
}
