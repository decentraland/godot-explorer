use godot::obj::Gd;

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

    fn get_font_bold_path(self) -> &'static str {
        match self {
            Font::FSansSerif => "res://assets/themes/fonts/noto/NotoSans-Bold.ttf",
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSans-Bold.ttf",
            Font::FMonospace => "res://assets/themes/fonts/noto/NotoSans-Bold.ttf",
        }
    }

    fn get_font_italic_path(self) -> &'static str {
        match self {
            Font::FSansSerif => "res://assets/themes/fonts/noto/NotoSans-Italic.ttf",
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSans-Italic.ttf",
            Font::FMonospace => "res://assets/themes/fonts/noto/NotoSans-Italic.ttf",
        }
    }

    fn get_font_bold_italic_path(self) -> &'static str {
        match self {
            Font::FSansSerif => "res://assets/themes/fonts/noto/NotoSans-BoldItalic.ttf",
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSans-BoldItalic.ttf",
            Font::FMonospace => "res://assets/themes/fonts/noto/NotoSans-BoldItalic.ttf",
        }
    }

    pub fn try_get_font_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_path()).ok()
    }

    pub fn try_get_font_bold_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_bold_path()).ok()
    }

    pub fn try_get_font_italic_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_italic_path()).ok()
    }

    pub fn try_get_font_bold_italic_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_bold_italic_path()).ok()
    }

    // TODO: Maybe the load can be cached and then return a duplicated one
    pub fn get_font_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_resource().expect("Failed to load font")
    }
    pub fn get_font_bold_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_bold_resource()
            .expect("Failed to load font")
    }
    pub fn get_font_italic_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_italic_resource()
            .expect("Failed to load font")
    }
    pub fn get_font_bold_italic_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_bold_italic_resource()
            .expect("Failed to load font")
    }
}

#[godot::test::itest]
fn test_font_load(_context: &TestContext) {
    assert_ne!(Font::FSansSerif.try_get_font_resource(), None);
    assert_ne!(Font::FSerif.try_get_font_resource(), None);
    assert_ne!(Font::FMonospace.try_get_font_resource(), None);
}
