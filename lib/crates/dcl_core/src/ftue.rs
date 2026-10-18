//! First-time guidance in the world: the controls overlay that labels the HUD on a scene entry
//! (#3014). The shell draws it; what shows, when, and what is remembered is decided here. Flag
//! names, defaults and analytics names are the 1.x ones, which dashboards and remote config share.

use std::sync::Mutex;

use serde_json::{json, Value};

use crate::{
    analytics::data_definition::{SegmentEvent, SegmentEventClickButton, SegmentEventScreenViewed},
    app::{CameraMode, HudSnapshot},
    flags,
    settings::{SettingKey, SettingValue, Settings},
};

const FLAG_OVERLAY: &str = "ftue-overlay";
const FLAG_LOCK_SECONDS: &str = "ftue-overlay-lock-seconds";
const FLAG_TAP_THROUGH: &str = "ftue-overlay-tap-through";
const FLAG_EXISTING_PLAYERS: &str = "ftue-overlay-existing-players";

const SCREEN_CONTROLS: &str = "CONTROLS_FTUE";

/// A HUD control the overlay labels, in its entrance order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ControlsFtueElement {
    Menu,
    Chat,
    Movement,
    Emotes,
}

impl ControlsFtueElement {
    pub const ALL: [Self; 4] = [Self::Menu, Self::Chat, Self::Movement, Self::Emotes];

    /// The id 1.x stored and reported.
    pub fn name(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Chat => "chat",
            Self::Movement => "movement",
            Self::Emotes => "emotes",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }
}

/// The remote tunables, already clamped to what 1.x accepted.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct FtueTuning {
    /// Between the last label appearing and the tap prompt; every touch is swallowed meanwhile.
    pub overlay_lock_seconds: f64,
    /// The closing tap also presses the control under it.
    pub overlay_tap_through: bool,
}

/// What a scene entry gets.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SceneEntryFtue {
    Nothing,
    Controls { elements: Vec<ControlsFtueElement> },
}

/// Everything one scene entry is decided on.
#[derive(Clone, Debug, Default)]
pub struct EntryFacts {
    pub forced_controls: bool,
    pub controls_enabled: bool,
    pub existing_player: bool,
    pub show_existing_players: bool,
    pub scene_loaded: bool,
    /// Landscape, no menu open, the HUD shown, the camera not cinematic.
    pub hud_usable: bool,
    pub joystick_visible: bool,
    pub shown: Vec<ControlsFtueElement>,
}

impl EntryFacts {
    fn visible(&self) -> Vec<ControlsFtueElement> {
        ControlsFtueElement::ALL
            .into_iter()
            .filter(|e| *e != ControlsFtueElement::Movement || self.joystick_visible)
            .collect()
    }
}

pub fn decide(facts: &EntryFacts) -> SceneEntryFtue {
    if !facts.scene_loaded || !facts.hud_usable {
        return SceneEntryFtue::Nothing;
    }
    if !facts.forced_controls
        && (!facts.controls_enabled || (facts.existing_player && !facts.show_existing_players))
    {
        return SceneEntryFtue::Nothing;
    }
    let elements: Vec<_> = facts
        .visible()
        .into_iter()
        .filter(|e| facts.forced_controls || !facts.shown.contains(e))
        .collect();
    if elements.is_empty() {
        SceneEntryFtue::Nothing
    } else {
        SceneEntryFtue::Controls { elements }
    }
}

pub fn tuning() -> FtueTuning {
    FtueTuning {
        overlay_lock_seconds: flags::get_number(FLAG_LOCK_SECONDS, 3.0).clamp(0.0, 10.0),
        overlay_tap_through: flags::is_enabled(FLAG_TAP_THROUGH, false),
    }
}

/// Developer link `controls-ftue=show`, for this session only: the next entry labels every
/// visible control, even ones already labelled.
static FORCED_CONTROLS: Mutex<bool> = Mutex::new(false);

fn forced_controls() -> std::sync::MutexGuard<'static, bool> {
    FORCED_CONTROLS.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn force_controls() {
    *forced_controls() = true;
}

/// What the rest of the core knows when the shell reports a scene entry.
pub struct EntryContext<'a> {
    pub hud: Option<&'a HudSnapshot>,
    /// Launches of this install, this one included.
    pub sessions: u32,
    /// A menu is open, the phone is in portrait or the HUD is hidden.
    pub blocked: bool,
}

fn text(settings: &Settings, key: SettingKey) -> String {
    match settings.get(key) {
        SettingValue::Text(text) => text,
        _ => String::new(),
    }
}

fn save(settings: &Settings, key: SettingKey, value: SettingValue) {
    if let Err(e) = settings.set(key, value, true) {
        tracing::warn!("ftue: could not save {}: {e}", key.name());
    }
}

pub fn shown(settings: &Settings) -> Vec<ControlsFtueElement> {
    text(settings, SettingKey::ControlsFtueShown)
        .split(',')
        .filter_map(ControlsFtueElement::from_name)
        .collect()
}

fn mark_shown(settings: &Settings, elements: &[ControlsFtueElement]) {
    let mut all = shown(settings);
    for element in elements {
        if !all.contains(element) {
            all.push(*element);
        }
    }
    let names: Vec<_> = all.iter().map(|e| e.name()).collect();
    save(
        settings,
        SettingKey::ControlsFtueShown,
        SettingValue::Text(names.join(",")),
    );
}

fn facts(settings: &Settings, ctx: &EntryContext, hud: &HudSnapshot) -> EntryFacts {
    EntryFacts {
        forced_controls: *forced_controls(),
        controls_enabled: flags::is_enabled(FLAG_OVERLAY, true),
        existing_player: ctx.sessions > 1,
        show_existing_players: flags::is_enabled(FLAG_EXISTING_PLAYERS, true),
        scene_loaded: hud.scene_base.is_some(),
        hud_usable: !ctx.blocked && hud.camera_mode != CameraMode::Cinematic,
        joystick_visible: hud.joystick_visible,
        shown: shown(settings),
    }
}

fn track(event: SegmentEvent) {
    if let Some(analytics) = crate::analytics::global::analytics() {
        analytics.queue_event(event);
    }
}

fn screen(name: &str, extra: Option<Value>) -> SegmentEvent {
    SegmentEvent::ScreenViewed(SegmentEventScreenViewed {
        screen_name: name.to_string(),
        extra_properties: extra.map(|v| v.to_string()),
    })
}

fn click(button: &str, screen: &str, extra: Option<Value>) -> SegmentEvent {
    SegmentEvent::ClickButton(SegmentEventClickButton {
        button_text: button.to_string(),
        screen_name: screen.to_string(),
        extra_properties: extra.map(|v| v.to_string()),
    })
}

fn element_names(elements: &[ControlsFtueElement]) -> Vec<&'static str> {
    elements.iter().map(|e| e.name()).collect()
}

fn dismiss_props(elements: &[ControlsFtueElement], seconds_shown: f64) -> Value {
    let seconds = (seconds_shown.max(0.0) * 10.0).round() / 10.0;
    json!({"seconds_shown": seconds, "elements": element_names(elements)})
}

/// Called once per scene entry, after the loading cover and with no modal up. Nothing is saved
/// until the player answers, so an app kill shows the same thing again.
pub fn scene_entered(settings: &Settings, ctx: EntryContext) -> SceneEntryFtue {
    let Some(hud) = ctx.hud else {
        return SceneEntryFtue::Nothing;
    };
    let decision = decide(&facts(settings, &ctx, hud));
    if let SceneEntryFtue::Controls { elements } = &decision {
        track(screen(
            SCREEN_CONTROLS,
            Some(json!({"elements": element_names(elements)})),
        ));
    }
    decision
}

/// The overlay was closed with a tap.
pub fn controls_dismissed(
    settings: &Settings,
    elements: &[ControlsFtueElement],
    seconds_shown: f64,
) {
    *forced_controls() = false;
    mark_shown(settings, elements);
    track(click(
        "dismiss",
        SCREEN_CONTROLS,
        Some(dismiss_props(elements, seconds_shown)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Parcel, Vec3};
    use ControlsFtueElement::*;

    fn fresh() -> EntryFacts {
        EntryFacts {
            controls_enabled: true,
            show_existing_players: true,
            scene_loaded: true,
            hud_usable: true,
            joystick_visible: true,
            ..EntryFacts::default()
        }
    }

    fn hud(base: Option<Parcel>) -> HudSnapshot {
        HudSnapshot {
            parcel: Parcel::new(0, 0),
            realm: "https://realm-provider.decentraland.org/main".into(),
            fps: 30.0,
            position: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            scene_base: base,
            camera_mode: CameraMode::ThirdPerson,
            joystick_visible: true,
        }
    }

    fn open() -> (tempfile::TempDir, Settings) {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::open(dir.path());
        (dir, settings)
    }

    fn ctx(hud: &HudSnapshot) -> EntryContext<'_> {
        EntryContext {
            hud: Some(hud),
            sessions: 1,
            blocked: false,
        }
    }

    #[test]
    fn a_first_entry_labels_every_visible_control() {
        assert_eq!(
            decide(&fresh()),
            SceneEntryFtue::Controls {
                elements: vec![Menu, Chat, Movement, Emotes]
            }
        );
    }

    #[test]
    fn only_controls_not_labelled_yet_and_on_screen_are_labelled() {
        let facts = EntryFacts {
            joystick_visible: false,
            ..fresh()
        };
        assert_eq!(
            decide(&facts),
            SceneEntryFtue::Controls {
                elements: vec![Menu, Chat, Emotes]
            }
        );
        let facts = EntryFacts {
            shown: vec![Menu, Chat, Emotes],
            ..fresh()
        };
        assert_eq!(
            decide(&facts),
            SceneEntryFtue::Controls {
                elements: vec![Movement]
            }
        );
        let facts = EntryFacts {
            shown: ControlsFtueElement::ALL.to_vec(),
            ..fresh()
        };
        assert_eq!(decide(&facts), SceneEntryFtue::Nothing);
    }

    #[test]
    fn the_overlay_waits_for_a_usable_hud_and_obeys_its_flags() {
        for facts in [
            EntryFacts {
                hud_usable: false,
                ..fresh()
            },
            EntryFacts {
                scene_loaded: false,
                ..fresh()
            },
            EntryFacts {
                controls_enabled: false,
                ..fresh()
            },
            EntryFacts {
                existing_player: true,
                show_existing_players: false,
                ..fresh()
            },
        ] {
            assert_eq!(decide(&facts), SceneEntryFtue::Nothing, "{facts:?}");
        }
    }

    #[test]
    fn the_dev_link_labels_everything_again_even_with_the_flag_off() {
        let facts = EntryFacts {
            forced_controls: true,
            controls_enabled: false,
            shown: ControlsFtueElement::ALL.to_vec(),
            ..fresh()
        };
        assert_eq!(
            decide(&facts),
            SceneEntryFtue::Controls {
                elements: ControlsFtueElement::ALL.to_vec()
            }
        );
    }

    #[test]
    fn dismiss_payload_keeps_the_1x_shape() {
        assert_eq!(
            dismiss_props(&[Menu, Movement], 3.4567),
            json!({"seconds_shown": 3.5, "elements": ["menu", "movement"]})
        );
    }

    #[test]
    fn dismissing_remembers_the_labels_once_per_install() {
        let (_dir, settings) = open();
        let scene = hud(Some(Parcel::new(40, 40)));
        assert!(matches!(
            scene_entered(&settings, ctx(&scene)),
            SceneEntryFtue::Controls { .. }
        ));
        controls_dismissed(&settings, &[Menu, Chat], 2.0);
        assert_eq!(shown(&settings), vec![Menu, Chat]);
        assert_eq!(
            scene_entered(&settings, ctx(&scene)),
            SceneEntryFtue::Controls {
                elements: vec![Movement, Emotes]
            }
        );
    }

    #[test]
    fn a_cinematic_camera_a_blocked_hud_or_no_report_shows_nothing() {
        let (_dir, settings) = open();
        let mut cinematic = hud(Some(Parcel::new(1, 1)));
        cinematic.camera_mode = CameraMode::Cinematic;
        assert_eq!(
            scene_entered(&settings, ctx(&cinematic)),
            SceneEntryFtue::Nothing
        );
        let plain = hud(Some(Parcel::new(1, 1)));
        let blocked = EntryContext {
            blocked: true,
            ..ctx(&plain)
        };
        assert_eq!(scene_entered(&settings, blocked), SceneEntryFtue::Nothing);
        let before_report = EntryContext {
            hud: None,
            ..ctx(&plain)
        };
        assert_eq!(
            scene_entered(&settings, before_report),
            SceneEntryFtue::Nothing
        );
        assert_eq!(
            scene_entered(&settings, ctx(&hud(None))),
            SceneEntryFtue::Nothing,
            "no scene loaded where the player stands"
        );
    }
}
