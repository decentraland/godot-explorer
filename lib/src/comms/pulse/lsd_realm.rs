//! Local Scene Development identity → the Pulse realm key.
//!
//! Pulse partitions visibility by **exact realm-string match**, and the key is never exchanged:
//! every party (this client, unity-explorer, bevy-explorer, sdk-commands and the orchestrator)
//! derives it independently from the preview scene's entity id. That is what makes a local
//! preview isolated with no handshake — and it is also the failure mode, because a derivation
//! that drifts does not error. The peers simply never see each other.
//!
//! Canonical contract: `docs/lsd-identity-and-pulse-realm.md` in js-sdk-toolchain. The two
//! worked examples published there are pinned as tests below; keep them passing or change every
//! implementation at once.

use sha2::{Digest, Sha256};

/// Pulse's `MaxRealmLength`. A longer realm is rejected with a terminal `InvalidTeleportField`.
pub const PULSE_MAX_REALM_LENGTH: usize = 255;

const LSD_REALM_PREFIX: &str = "lsd:";
const LSD_REALM_HASHED_PREFIX: &str = "lsd:sha256:";

/// The Pulse realm key for a Local Scene Development preview: `lsd:` + the preview scene's
/// entity id (`b64-<base64(absoluteProjectRoot + "-" + machineId)>`, which the preview server
/// already serves — this client reuses it verbatim rather than re-deriving the base64).
///
/// Once the plain form would exceed [`PULSE_MAX_REALM_LENGTH`] it collapses to
/// `lsd:sha256:<lowercase hex SHA-256 of the entity id, `b64-` prefix included>`, always 75
/// characters. Hashed and not truncated, so every party lands on the identical string without
/// coordinating.
pub fn lsd_realm_key(preview_scene_id: &str) -> String {
    let key = format!("{LSD_REALM_PREFIX}{preview_scene_id}");
    if key.len() <= PULSE_MAX_REALM_LENGTH {
        return key;
    }
    let digest = Sha256::digest(preview_scene_id.as_bytes());
    format!("{LSD_REALM_HASHED_PREFIX}{}", hex::encode(digest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    /// `b64HashingFunction` from sdk-commands `logic/project-files.ts`, reproduced only to build
    /// the published vectors — at runtime the id arrives from the preview server.
    fn preview_scene_id(project_root: &str, machine_id: &str) -> String {
        format!(
            "b64-{}",
            base64::engine::general_purpose::STANDARD
                .encode(format!("{project_root}-{machine_id}"))
        )
    }

    /// First worked example in `docs/lsd-identity-and-pulse-realm.md`.
    #[test]
    fn plain_key_matches_the_published_vector() {
        let id = preview_scene_id("/home/dev/my-scene", "dev-box");
        assert_eq!(id, "b64-L2hvbWUvZGV2L215LXNjZW5lLWRldi1ib3g=");
        assert_eq!(
            lsd_realm_key(&id),
            "lsd:b64-L2hvbWUvZGV2L215LXNjZW5lLWRldi1ib3g="
        );
    }

    /// Second worked example: a 300-character raw key collapses to the hashed form.
    #[test]
    fn oversized_key_matches_the_published_vector() {
        let id = preview_scene_id(&format!("/home/dev/{}", "a".repeat(200)), "dev-box");
        assert_eq!(LSD_REALM_PREFIX.len() + id.len(), 300);
        assert_eq!(
            lsd_realm_key(&id),
            "lsd:sha256:783635fb50eadaed0300d80104920bfc55894d5ad2ab69ab6b48c6ff1ddb9da5"
        );
    }

    /// The hashed form is always 75 chars, so it always fits.
    #[test]
    fn hashed_form_fits_the_realm_cap() {
        let key = lsd_realm_key(&"b64-".repeat(200));
        assert_eq!(key.len(), 75);
        assert!(key.len() <= PULSE_MAX_REALM_LENGTH);
    }

    /// The boundary is inclusive: exactly 255 stays plain, 256 collapses.
    #[test]
    fn boundary_is_inclusive() {
        let exact = "x".repeat(PULSE_MAX_REALM_LENGTH - LSD_REALM_PREFIX.len());
        assert_eq!(lsd_realm_key(&exact).len(), PULSE_MAX_REALM_LENGTH);
        assert!(lsd_realm_key(&exact).starts_with(LSD_REALM_PREFIX));

        let over = "x".repeat(PULSE_MAX_REALM_LENGTH - LSD_REALM_PREFIX.len() + 1);
        assert!(lsd_realm_key(&over).starts_with(LSD_REALM_HASHED_PREFIX));
    }
}
