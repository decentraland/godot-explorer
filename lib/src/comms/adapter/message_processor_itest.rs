//! Integration tests for the comms [`MessageProcessor`] driven against a real
//! [`AvatarScene`] with **synthetic** RFC4 packets — no network, no LiveKit.
//!
//! These run inside headless Godot via `cargo run -- run --itest` (collected by
//! the runtime `#[godot::test::itest]` registry, like `notifications.rs`). They
//! cover the MessageProcessor → AvatarScene path that pure `cargo test` can't
//! reach: avatars appearing/moving/emoting/leaving in response to packets.

// The module is always compiled (the itest registry is enumerated at runtime by
// the Godot test-runner). The bodies only run under `--itest`.
mod message_processor_itests {
    use crate::avatars::avatar_scene::AvatarScene;
    use crate::comms::adapter::message_processor::{
        IncomingMessage, MessageProcessor, MessageType, Rfc4Message,
    };
    use crate::dcl::components::proto_components::kernel::comms::rfc4;
    use crate::framework::TestContext;
    use ethers_core::types::H160;
    use godot::prelude::*;

    const ROOM: &str = "itest-room";

    /// Local player + a remote peer address (must be > 0xff to be a "real"
    /// player address — see `MessageProcessor::is_player_address`).
    fn local_addr() -> H160 {
        H160::from_low_u64_be(0x1111_0000)
    }
    fn peer_addr() -> H160 {
        H160::from_low_u64_be(0xabcd_ef01_2345)
    }

    /// A fixture owning a real AvatarScene (parented into the test tree) and a
    /// MessageProcessor wired to it. `inject` pushes a synthetic packet and polls.
    struct Fixture {
        avatars: Gd<AvatarScene>,
        processor: MessageProcessor,
        sender: tokio::sync::mpsc::Sender<IncomingMessage>,
    }

    impl Fixture {
        fn new(ctx: &TestContext, scene_name: &str) -> Self {
            let mut avatars = AvatarScene::new_alloc();
            avatars.set_name(scene_name);
            // Parent into the test tree so avatar nodes get _ready'd.
            ctx.scene_tree.clone().add_child(&avatars);

            let processor = MessageProcessor::new(local_addr(), None, avatars.clone());
            let sender = processor.get_message_sender();
            Self {
                avatars,
                processor,
                sender,
            }
        }

        fn inject_rfc4(&mut self, from: H160, message: rfc4::packet::Message) {
            self.sender
                .try_send(IncomingMessage {
                    message: MessageType::Rfc4(Rfc4Message {
                        message,
                        protocol_version: 100,
                    }),
                    address: from,
                    room_id: ROOM.to_string(),
                })
                .expect("inject rfc4");
            self.processor.poll();
        }

        fn inject(&mut self, from: H160, message: MessageType) {
            self.sender
                .try_send(IncomingMessage {
                    message,
                    address: from,
                    room_id: ROOM.to_string(),
                })
                .expect("inject");
            self.processor.poll();
        }

        fn count(&self) -> i64 {
            self.avatars.bind().get_avatars_count() as i64
        }

        /// Alias the MessageProcessor assigned to the first/only peer is 1
        /// (peer_alias_counter starts at 0 and is pre-incremented).
        fn first_alias() -> i64 {
            1
        }

        fn last_emote(&self) -> String {
            self.avatars
                .bind()
                .debug_last_emote(Self::first_alias())
                .to_string()
        }

        fn last_movement_ts(&self) -> f32 {
            self.avatars
                .bind()
                .debug_avatar_last_movement_ts(Self::first_alias())
        }

        fn teardown(mut self) {
            self.processor.clean();
            self.avatars.clone().queue_free();
        }
    }

    fn movement(timestamp: f32, x: f32, y: f32, z: f32) -> rfc4::packet::Message {
        rfc4::packet::Message::Movement(rfc4::Movement {
            timestamp,
            position_x: x,
            position_y: y,
            position_z: z,
            ..Default::default()
        })
    }

    fn emote(incremental_id: u32, urn: &str, timestamp: f32) -> rfc4::packet::Message {
        rfc4::packet::Message::PlayerEmote(rfc4::PlayerEmote {
            incremental_id,
            urn: urn.to_string(),
            timestamp,
        })
    }

    // ---- appearance ----

    #[godot::test::itest]
    fn peer_appears_on_first_packet(ctx: &TestContext) {
        let mut fx = Fixture::new(ctx, "mp_appear");
        assert_eq!(fx.count(), 0, "no avatars before any packet");

        fx.inject_rfc4(peer_addr(), movement(1.0, 5.0, 0.0, 5.0));
        assert_eq!(fx.count(), 1, "avatar created for new peer");

        // A second packet from the same peer must NOT create a duplicate avatar.
        fx.inject_rfc4(peer_addr(), movement(2.0, 6.0, 0.0, 5.0));
        assert_eq!(fx.count(), 1, "same peer does not duplicate");

        fx.teardown();
    }

    #[godot::test::itest]
    fn peer_removed_on_peer_left(ctx: &TestContext) {
        let mut fx = Fixture::new(ctx, "mp_left");
        fx.inject_rfc4(peer_addr(), movement(1.0, 5.0, 0.0, 5.0));
        assert_eq!(fx.count(), 1);

        fx.inject(peer_addr(), MessageType::PeerLeft);
        assert_eq!(
            fx.count(),
            0,
            "avatar removed when peer leaves its only room"
        );

        fx.teardown();
    }

    // ---- movement / position ----

    #[godot::test::itest]
    fn movement_is_accepted_and_deduped(ctx: &TestContext) {
        // Avatars move via frame-stepped lerp, so the rendered position can't be
        // asserted synchronously. Instead assert the MessageProcessor processed
        // the Movement and applied it to the avatar (last accepted timestamp),
        // and that stale/duplicate movements are dropped. Position *math* is
        // covered by movement_compressed's round-trip unit tests.
        let mut fx = Fixture::new(ctx, "mp_move");
        fx.inject_rfc4(peer_addr(), movement(1.0, 12.0, 0.0, 34.0));
        assert_eq!(fx.last_movement_ts(), 1.0, "first movement accepted");

        // A newer movement is accepted.
        fx.inject_rfc4(peer_addr(), movement(2.0, -40.0, 0.0, 8.0));
        assert_eq!(fx.last_movement_ts(), 2.0, "newer movement accepted");

        // A duplicate (same timestamp) is dropped.
        fx.inject_rfc4(peer_addr(), movement(2.0, 7.0, 0.0, 7.0));
        assert_eq!(fx.last_movement_ts(), 2.0, "duplicate timestamp dropped");

        // A stale (older timestamp) movement is dropped.
        fx.inject_rfc4(peer_addr(), movement(1.5, 999.0, 0.0, 999.0));
        assert_eq!(fx.last_movement_ts(), 2.0, "stale movement dropped");

        fx.teardown();
    }

    // ---- emote (regression for the incremental_id == 0 drop) ----

    #[godot::test::itest]
    fn emote_with_incremental_id_zero_plays(ctx: &TestContext) {
        let mut fx = Fixture::new(ctx, "mp_emote0");
        fx.inject_rfc4(peer_addr(), movement(1.0, 0.0, 0.0, 0.0)); // create the avatar
        assert_eq!(fx.last_emote(), "", "no emote yet");

        // The exact shape of the bug: a web/Foundation client emote with id == 0.
        fx.inject_rfc4(peer_addr(), emote(0, "wave", 10.0));
        assert_eq!(fx.last_emote(), "wave", "id=0 emote must play (regression)");

        // Re-broadcast of the same emote (newer ts, same urn/id) does not re-fire.
        fx.inject_rfc4(peer_addr(), emote(0, "wave", 12.0));
        assert_eq!(fx.last_emote(), "wave");

        // Switching emote (still id=0) updates it.
        fx.inject_rfc4(peer_addr(), emote(0, "kiss", 14.0));
        assert_eq!(fx.last_emote(), "kiss", "id=0 urn switch must play");

        fx.teardown();
    }

    #[godot::test::itest]
    fn emote_with_incrementing_id_plays(ctx: &TestContext) {
        let mut fx = Fixture::new(ctx, "mp_emoteN");
        fx.inject_rfc4(peer_addr(), movement(1.0, 0.0, 0.0, 0.0));

        fx.inject_rfc4(peer_addr(), emote(1, "dance", 10.0));
        assert_eq!(fx.last_emote(), "dance");

        // Same urn, higher id => a new trigger, plays again (we can't see the
        // re-fire directly, but a later different urn must win).
        fx.inject_rfc4(peer_addr(), emote(5, "clap", 20.0));
        assert_eq!(fx.last_emote(), "clap");

        fx.teardown();
    }
}
