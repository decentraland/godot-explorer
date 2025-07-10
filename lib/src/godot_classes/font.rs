use godot::{classes::ResourceLoader, obj::Gd};

use crate::{
    dcl::components::proto_components::sdk::components::common::Font, framework::TestContext,
};

impl Font {
    fn get_font_path(self) -> &'static str {
        match self {
            Font::FSansSerif => "res://assets/themes/fonts/noto/NotoSans-Regular.ttf",
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSerif-Regular.ttf",
            Font::FMonospace => "res://assets/themes/fonts/noto/NotoSansMono-Regular.ttf",
        }
    }

    pub fn try_get_font_resource(&self) -> Option<Gd<godot::classes::Font>> {
        ResourceLoader::singleton()
            .load(self.get_font_path())
            .map(|resource| resource.cast::<godot::classes::Font>())
    }

    // TODO: Maybe the load can be cached and then return a duplicated one
    pub fn get_font_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_resource().expect("Failed to load font")
    }
}

#[godot::test::itest]
fn test_font_load(_context: &TestContext) {
    assert_ne!(Font::FSansSerif.try_get_font_resource(), None);
    assert_ne!(Font::FSerif.try_get_font_resource(), None);
    assert_ne!(Font::FMonospace.try_get_font_resource(), None);
}
