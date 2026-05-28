use godot::obj::Singleton;

use crate::framework::TestContext;

/// Locks the UI design base resolution to 1600x720. `graphic_settings.gd`
/// reads `display/window/size/viewport_*` to compute the UI zoom factor, so an
/// accidental edit to `project.godot` would silently distort the HUD scale on
/// every platform. If this fails, intentionally update both the project
/// settings and this test together.
#[godot::test::itest]
fn test_viewport_base_resolution_is_1600x720(_ctx: &TestContext) {
    let settings = godot::classes::ProjectSettings::singleton();

    let width = settings
        .get_setting("display/window/size/viewport_width")
        .to::<i64>();
    let height = settings
        .get_setting("display/window/size/viewport_height")
        .to::<i64>();

    assert_eq!(
        width, 1600,
        "display/window/size/viewport_width must stay 1600"
    );
    assert_eq!(
        height, 720,
        "display/window/size/viewport_height must stay 720"
    );
}
