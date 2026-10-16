//! The sign-out: the single teardown every logout funnels through.

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

/// Tears the live session down. Guarded against re-entrancy: the comms close emits the logout
/// signal deferred, which can route back here a frame later and would otherwise run the teardown
/// and the scene swap twice.
pub fn sign_out() {
    let global = DclGlobal::singleton();
    if global.bind().signing_out {
        return;
    }
    global.clone().bind_mut().signing_out = true;

    // 1. The live explorer first, while it is still in the tree, so its signal callbacks and retry
    //    timers are severed before the steps below re-emit any of those signals (orientation, the
    //    realm clear, the scene kill) or free it.
    crate::world::api::prepare_for_logout();

    // 2. The session's background work. The analytics first-move poll reads the player body; left
    //    running it would poll the freed player after the swap. It restarts on the next login.
    if let Some(mut analytics) = global.bind().analytics_controller.clone() {
        analytics.bind_mut().cancel_first_move_poll();
    }
    let mut blacklist = global.bind().social_blacklist.clone();
    blacklist.bind_mut().clear_blocked();
    blacklist.bind_mut().clear_muted();

    // 3. Every running scene: they are marked to die and the runner reaps them in the background,
    //    tearing down their V8 threads.
    globals::scene_runner().bind_mut().kill_all_scenes();

    // 4. The realm and the fetcher, so nothing of the old session leaks into the next login. The
    //    realm clear runs synchronously: it only zeroes fields and kills scenes.
    globals::realm().bind_mut().async_clear_realm();
    if let Some(mut fetcher) = global.bind().scene_fetcher.clone() {
        fetcher.bind_mut().reset_for_logout();
    }

    // 5. Comms, and the player identity (wallet and ephemeral keys). Idempotent: a second logout
    //    does nothing once the address is cleared.
    globals::comms().bind_mut().leave(true);

    // 6. Forget the previous identity so the next account starts fresh: otherwise the prior profile
    //    and avatar linger in memory and leak into the new account's avatar editor (#1658), and the
    //    saved guest look would carry from guest to guest.
    globals::player_identity().bind_mut().reset_identity();

    // The login is portrait only: logging out of a landscape screen must not strand it there.
    DclGlobal::set_orientation_portrait();

    // 7. The scene runner's base UI is detached from the dying explorer so freeing it cannot free
    //    UI nodes Rust still references: a cache-hot scene spawn racing the next explorer ready
    //    would panic on the freed control (GODOT-EXPLORER-1DY). The orphan is freed by the next
    //    `recreate_base_ui`.
    globals::scene_runner().bind_mut().detach_base_ui();

    global.clone().bind_mut().signing_out = false;
}
