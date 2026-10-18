//! First-time guidance in the world: the controls overlay that labels the HUD on a scene entry
//! (#3014) and the guided tutorial offered to a player whose account was created on this install
//! (#2767). The shell draws both; what shows, when, and what is remembered is decided here. A
//! player gets one or the other, whichever they answer first. Flag names, defaults and analytics
//! names are the 1.x ones, which dashboards and remote config share.

use std::sync::Mutex;

use serde_json::{json, Value};

use crate::{
    analytics::data_definition::{SegmentEvent, SegmentEventClickButton, SegmentEventScreenViewed},
    app::{CameraMode, HudSnapshot, Parcel},
    flags, realm,
    settings::{SettingKey, SettingValue, Settings},
};

const FLAG_OVERLAY: &str = "ftue-overlay";
const FLAG_LOCK_SECONDS: &str = "ftue-overlay-lock-seconds";
const FLAG_TAP_THROUGH: &str = "ftue-overlay-tap-through";
const FLAG_EXISTING_PLAYERS: &str = "ftue-overlay-existing-players";
const FLAG_TUTORIAL: &str = "ftue-tutorial";
const FLAG_SCENES: &str = "ftue-tutorial-scenes";
const FLAG_MOVE_METERS: &str = "ftue-tutorial-move-meters";
const FLAG_CAMERA_DEGREES: &str = "ftue-tutorial-camera-degrees";
const FLAG_SOCIAL_SECONDS: &str = "ftue-tutorial-social-seconds";
const FLAG_STEP_DELAY_SECONDS: &str = "ftue-tutorial-step-delay-seconds";
const FLAG_CHAT_SECONDS: &str = "ftue-tutorial-chat-seconds";

/// Genesis Plaza's base parcel, as `x,y;x,y`.
const DEFAULT_SCENES: &str = "-3,-2";

const SCREEN_CONTROLS: &str = "CONTROLS_FTUE";
const SCREEN_WELCOME: &str = "FTUE_TUTORIAL_WELCOME";
const SCREEN_STEP: &str = "FTUE_TUTORIAL_STEP";
const SCREEN_COMPLETE: &str = "FTUE_TUTORIAL_COMPLETE";
const SCREEN_SETTINGS: &str = "SETTINGS";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum TutorialStep {
    Move,
    Camera,
    Pinch,
    Chat,
    Menu,
    Social,
    Backpack,
    Equip,
}

impl TutorialStep {
    pub fn name(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Camera => "camera",
            Self::Pinch => "pinch",
            Self::Chat => "chat",
            Self::Menu => "menu",
            Self::Social => "social",
            Self::Backpack => "backpack",
            Self::Equip => "equip",
        }
    }

    /// The step the player sees: opening the Backpack and equipping in it are one.
    pub fn number(self) -> u32 {
        match self {
            Self::Move => 1,
            Self::Camera => 2,
            Self::Pinch => 3,
            Self::Chat => 4,
            Self::Menu => 5,
            Self::Social => 6,
            Self::Backpack | Self::Equip => 7,
        }
    }
}

/// The remote tunables of both, already clamped to what 1.x accepted.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct FtueTuning {
    /// Between the last label appearing and the tap prompt; every touch is swallowed meanwhile.
    pub overlay_lock_seconds: f64,
    /// The closing tap also presses the control under it.
    pub overlay_tap_through: bool,
    pub move_meters: f64,
    pub camera_degrees: f64,
    /// How long Social stays open before the Backpack step.
    pub social_seconds: f64,
    /// Pause with nothing on screen after the move, camera and pinch steps.
    pub step_delay_seconds: f64,
    /// After the chat opens, how long touches are ignored so a stray tap cannot close it.
    pub chat_seconds: f64,
}

/// What a scene entry gets.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SceneEntryFtue {
    Nothing,
    /// The welcome card that offers the tutorial.
    Tutorial,
    Controls {
        elements: Vec<ControlsFtueElement>,
    },
}

/// Everything one scene entry is decided on.
#[derive(Clone, Debug, Default)]
pub struct EntryFacts {
    pub forced_controls: bool,
    pub forced_tutorial: bool,
    pub controls_enabled: bool,
    pub existing_player: bool,
    pub show_existing_players: bool,
    pub tutorial_enabled: bool,
    pub new_player: bool,
    pub tutorial_offered: bool,
    pub tutorial_scene: bool,
    pub scene_loaded: bool,
    /// Landscape, no menu open, the HUD shown, the camera not cinematic.
    pub hud_usable: bool,
    pub joystick_visible: bool,
    pub shown: Vec<ControlsFtueElement>,
}

impl EntryFacts {
    /// The tutorial is due here, so the overlay stays away even if the card cannot show yet.
    fn tutorial_owns_entry(&self) -> bool {
        self.forced_tutorial
            || (self.tutorial_enabled
                && self.new_player
                && self.tutorial_scene
                && !self.tutorial_offered)
    }

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
    if facts.tutorial_owns_entry() {
        // Step 1 points at the joystick.
        return if facts.joystick_visible {
            SceneEntryFtue::Tutorial
        } else {
            SceneEntryFtue::Nothing
        };
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

/// `x,y;x,y`, skipping anything malformed.
pub fn parse_scenes(text: &str) -> Vec<Parcel> {
    text.split(';')
        .filter_map(|pair| {
            let (x, y) = pair.trim().split_once(',')?;
            Some(Parcel::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}

pub fn tuning() -> FtueTuning {
    let number =
        |name, default: f64, min: f64, max: f64| flags::get_number(name, default).clamp(min, max);
    FtueTuning {
        overlay_lock_seconds: number(FLAG_LOCK_SECONDS, 3.0, 0.0, 10.0),
        overlay_tap_through: flags::is_enabled(FLAG_TAP_THROUGH, false),
        move_meters: number(FLAG_MOVE_METERS, 1.0, 0.1, 20.0),
        camera_degrees: number(FLAG_CAMERA_DEGREES, 30.0, 1.0, 360.0),
        social_seconds: number(FLAG_SOCIAL_SECONDS, 3.0, 0.0, 10.0),
        step_delay_seconds: number(FLAG_STEP_DELAY_SECONDS, 2.0, 0.0, 10.0),
        chat_seconds: number(FLAG_CHAT_SECONDS, 3.0, 0.0, 10.0),
    }
}

#[derive(Default)]
struct Forced {
    controls: bool,
    tutorial: bool,
}

/// Developer links, for this session only.
static FORCED: Mutex<Forced> = Mutex::new(Forced {
    controls: false,
    tutorial: false,
});

fn forced() -> std::sync::MutexGuard<'static, Forced> {
    FORCED.lock().unwrap_or_else(|e| e.into_inner())
}

/// `controls-ftue=show`: the next entry labels every visible control, even ones already labelled.
pub fn force_controls() {
    forced().controls = true;
}

/// `ftue-tutorial=start`: the next entry offers the tutorial to any account, anywhere.
pub fn force_tutorial() {
    forced().tutorial = true;
}

/// What the rest of the core knows when the shell reports a scene entry.
pub struct EntryContext<'a> {
    pub hud: Option<&'a HudSnapshot>,
    /// The signed-in address.
    pub address: Option<String>,
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

fn flag(settings: &Settings, key: SettingKey) -> bool {
    settings.get(key) == SettingValue::Bool(true)
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

fn is_new_player(settings: &Settings, address: Option<&str>) -> bool {
    let created_here = text(settings, SettingKey::FtueNewPlayerWallet);
    !created_here.is_empty() && address.is_some_and(|a| a.to_lowercase() == created_here)
}

fn facts(settings: &Settings, ctx: &EntryContext, hud: &HudSnapshot) -> EntryFacts {
    let forced = forced();
    let scenes = parse_scenes(&flags::get_text(FLAG_SCENES, DEFAULT_SCENES));
    EntryFacts {
        forced_controls: forced.controls,
        forced_tutorial: forced.tutorial,
        controls_enabled: flags::is_enabled(FLAG_OVERLAY, true),
        existing_player: ctx.sessions > 1,
        show_existing_players: flags::is_enabled(FLAG_EXISTING_PLAYERS, true),
        tutorial_enabled: flags::is_enabled(FLAG_TUTORIAL, true),
        new_player: is_new_player(settings, ctx.address.as_deref()),
        tutorial_offered: flag(settings, SettingKey::FtueTutorialOffered),
        // A world can declare any base parcel, so the parcel alone does not identify a scene.
        tutorial_scene: realm::is_genesis_city(&hud.realm)
            && hud.scene_base.is_some_and(|base| scenes.contains(&base)),
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

fn step_props(step: TutorialStep, is_replay: bool) -> Value {
    json!({"step": step.number(), "step_name": step.name(), "is_replay": is_replay})
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
    match &decision {
        SceneEntryFtue::Tutorial => {
            forced().tutorial = false;
            track(screen(SCREEN_WELCOME, None));
        }
        SceneEntryFtue::Controls { elements } => track(screen(
            SCREEN_CONTROLS,
            Some(json!({"elements": element_names(elements)})),
        )),
        SceneEntryFtue::Nothing => {}
    }
    decision
}

/// The overlay was closed with a tap. The tutorial is not offered after it.
pub fn controls_dismissed(
    settings: &Settings,
    elements: &[ControlsFtueElement],
    seconds_shown: f64,
) {
    forced().controls = false;
    mark_shown(settings, elements);
    if !flag(settings, SettingKey::FtueTutorialOffered) {
        save(
            settings,
            SettingKey::FtueTutorialOffered,
            SettingValue::Bool(true),
        );
    }
    track(click(
        "dismiss",
        SCREEN_CONTROLS,
        Some(dismiss_props(elements, seconds_shown)),
    ));
}

/// START or SKIP on the welcome card. Either answer means the overlay never follows; START alone
/// does not settle the offer, so a run cut short by closing the app is offered again.
pub fn welcome_answered(settings: &Settings, start: bool) {
    mark_shown(settings, &ControlsFtueElement::ALL);
    if !start {
        save(
            settings,
            SettingKey::FtueTutorialOffered,
            SettingValue::Bool(true),
        );
    }
    track(click(
        if start { "start" } else { "skip" },
        SCREEN_WELCOME,
        Some(json!({"step": 0})),
    ));
}

pub fn step_entered(step: TutorialStep, is_replay: bool) {
    track(screen(SCREEN_STEP, Some(step_props(step, is_replay))));
}

pub fn skipped(settings: &Settings, step: TutorialStep, is_replay: bool) {
    save(
        settings,
        SettingKey::FtueTutorialOffered,
        SettingValue::Bool(true),
    );
    track(click(
        "skip",
        SCREEN_STEP,
        Some(step_props(step, is_replay)),
    ));
}

pub fn completed(settings: &Settings, is_replay: bool) {
    save(
        settings,
        SettingKey::FtueTutorialOffered,
        SettingValue::Bool(true),
    );
    save(
        settings,
        SettingKey::FtueTutorialCompleted,
        SettingValue::Bool(true),
    );
    mark_shown(settings, &ControlsFtueElement::ALL);
    track(screen(
        SCREEN_COMPLETE,
        Some(json!({"is_replay": is_replay})),
    ));
}

pub fn replay_requested() {
    track(click("replay_tutorial", SCREEN_SETTINGS, None));
}

/// A brand-new account finished its avatar: it is owed the tutorial, even if an earlier account
/// on this install was already offered it.
pub fn mark_new_player(settings: &Settings, address: &str) {
    save(
        settings,
        SettingKey::FtueNewPlayerWallet,
        SettingValue::Text(address.to_lowercase()),
    );
    save(
        settings,
        SettingKey::FtueTutorialOffered,
        SettingValue::Bool(false),
    );
    save(
        settings,
        SettingKey::FtueTutorialCompleted,
        SettingValue::Bool(false),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Vec3;
    use ControlsFtueElement::*;

    const PLAZA: &str = "https://realm-provider.decentraland.org/main";

    fn fresh() -> EntryFacts {
        EntryFacts {
            controls_enabled: true,
            show_existing_players: true,
            tutorial_enabled: true,
            scene_loaded: true,
            hud_usable: true,
            joystick_visible: true,
            ..EntryFacts::default()
        }
    }

    fn hud(realm: &str, base: Option<Parcel>) -> HudSnapshot {
        HudSnapshot {
            parcel: Parcel::new(0, 0),
            realm: realm.into(),
            fps: 30.0,
            position: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            scene_base: base,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            camera_mode: CameraMode::ThirdPerson,
            camera_mode_locked: false,
            joystick_visible: true,
        }
    }

    fn open() -> (tempfile::TempDir, Settings) {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::open(dir.path());
        (dir, settings)
    }

    fn ctx<'a>(hud: &'a HudSnapshot, address: &str) -> EntryContext<'a> {
        EntryContext {
            hud: Some(hud),
            address: Some(address.into()),
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
    fn a_new_player_in_a_tutorial_scene_gets_the_tutorial_not_the_overlay() {
        let new_player = EntryFacts {
            new_player: true,
            tutorial_scene: true,
            ..fresh()
        };
        assert_eq!(decide(&new_player), SceneEntryFtue::Tutorial);

        let no_joystick = EntryFacts {
            joystick_visible: false,
            ..new_player.clone()
        };
        assert_eq!(decide(&no_joystick), SceneEntryFtue::Nothing);

        let offered = EntryFacts {
            tutorial_offered: true,
            ..new_player.clone()
        };
        assert!(matches!(decide(&offered), SceneEntryFtue::Controls { .. }));

        let elsewhere = EntryFacts {
            tutorial_scene: false,
            ..new_player
        };
        assert!(matches!(
            decide(&elsewhere),
            SceneEntryFtue::Controls { .. }
        ));

        let existing = EntryFacts {
            tutorial_scene: true,
            ..fresh()
        };
        assert!(matches!(decide(&existing), SceneEntryFtue::Controls { .. }));
    }

    #[test]
    fn scenes_flag_parses_and_skips_garbage() {
        assert_eq!(
            parse_scenes("-3,-2; 10,4;x,1;5;"),
            vec![Parcel::new(-3, -2), Parcel::new(10, 4)]
        );
        assert!(parse_scenes("").is_empty());
    }

    #[test]
    fn step_numbers_fold_the_equip_step_into_the_backpack() {
        assert_eq!(
            step_props(TutorialStep::Equip, true),
            json!({"step": 7, "step_name": "equip", "is_replay": true})
        );
        assert_eq!(TutorialStep::Move.number(), 1);
        assert_eq!(
            dismiss_props(&[Menu, Movement], 3.4567),
            json!({"seconds_shown": 3.5, "elements": ["menu", "movement"]})
        );
    }

    #[test]
    fn dismissing_remembers_the_labels_and_settles_the_tutorial() {
        let (_dir, settings) = open();
        mark_new_player(&settings, "0xABC");
        let plaza = hud(PLAZA, Some(Parcel::new(-3, -2)));
        let elsewhere = hud(PLAZA, Some(Parcel::new(40, 40)));

        let entry = scene_entered(&settings, ctx(&elsewhere, "0xabc"));
        assert!(matches!(entry, SceneEntryFtue::Controls { .. }));
        controls_dismissed(&settings, &[Menu, Chat], 2.0);
        assert_eq!(shown(&settings), vec![Menu, Chat]);
        assert_eq!(
            scene_entered(&settings, ctx(&plaza, "0xabc")),
            SceneEntryFtue::Controls {
                elements: vec![Movement, Emotes]
            },
            "the tutorial is not offered once the overlay was answered"
        );
    }

    #[test]
    fn the_tutorial_is_offered_to_the_account_made_here_until_answered() {
        let (_dir, settings) = open();
        let plaza = hud(PLAZA, Some(Parcel::new(-3, -2)));
        let world = hud(
            "https://worlds-content-server.decentraland.org/world/x.dcl.eth",
            Some(Parcel::new(-3, -2)),
        );
        assert!(
            !matches!(
                scene_entered(&settings, ctx(&plaza, "0xabc")),
                SceneEntryFtue::Tutorial
            ),
            "an account recovered here is not new"
        );

        mark_new_player(&settings, "0xABC");
        assert!(!matches!(
            scene_entered(&settings, ctx(&world, "0xabc")),
            SceneEntryFtue::Tutorial
        ));
        assert!(!matches!(
            scene_entered(&settings, ctx(&plaza, "0xdef")),
            SceneEntryFtue::Tutorial
        ));
        assert_eq!(
            scene_entered(&settings, ctx(&plaza, "0xabc")),
            SceneEntryFtue::Tutorial
        );

        welcome_answered(&settings, true);
        assert_eq!(shown(&settings), ControlsFtueElement::ALL.to_vec());
        assert_eq!(
            scene_entered(&settings, ctx(&plaza, "0xabc")),
            SceneEntryFtue::Tutorial,
            "START alone does not settle it"
        );
        skipped(&settings, TutorialStep::Chat, false);
        assert_eq!(
            scene_entered(&settings, ctx(&plaza, "0xabc")),
            SceneEntryFtue::Nothing
        );

        mark_new_player(&settings, "0xabc");
        assert_eq!(
            scene_entered(&settings, ctx(&plaza, "0xabc")),
            SceneEntryFtue::Tutorial,
            "a new account on this install is owed it again"
        );
        completed(&settings, false);
        assert_eq!(
            settings.get(SettingKey::FtueTutorialCompleted),
            SettingValue::Bool(true)
        );
        assert_eq!(
            scene_entered(&settings, ctx(&plaza, "0xabc")),
            SceneEntryFtue::Nothing
        );
    }

    #[test]
    fn a_cinematic_camera_or_a_blocked_hud_shows_nothing() {
        let (_dir, settings) = open();
        let mut cinematic = hud(PLAZA, Some(Parcel::new(1, 1)));
        cinematic.camera_mode = CameraMode::Cinematic;
        assert_eq!(
            scene_entered(&settings, ctx(&cinematic, "0xabc")),
            SceneEntryFtue::Nothing
        );
        let plain = hud(PLAZA, Some(Parcel::new(1, 1)));
        let blocked = EntryContext {
            blocked: true,
            ..ctx(&plain, "0xabc")
        };
        assert_eq!(scene_entered(&settings, blocked), SceneEntryFtue::Nothing);
        let before_report = EntryContext {
            hud: None,
            ..ctx(&plain, "0xabc")
        };
        assert_eq!(
            scene_entered(&settings, before_report),
            SceneEntryFtue::Nothing
        );
    }
}
