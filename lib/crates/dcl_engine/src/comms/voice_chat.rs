use godot::{
    classes::{
        AudioEffectCapture, AudioServer, AudioStreamMicrophone, AudioStreamPlayer,
        IAudioStreamPlayer, Input, ProjectSettings,
    },
    prelude::*,
};

use crate::fetcher::globals;
use crate::ui::ui_sounds;

const RECORD_ACTION: &str = "ia_record_mic";

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct VoiceChatRecorder {
    recording_enabled: bool,
    /// The record-mic action is held.
    is_recording: bool,
    /// The current adapter carries voice.
    voice_enabled: bool,
    effect_capture: Option<Gd<AudioEffectCapture>>,

    base: Base<AudioStreamPlayer>,
}

#[godot_api]
impl IAudioStreamPlayer for VoiceChatRecorder {
    fn ready(&mut self) {
        if !ProjectSettings::singleton()
            .get_setting_with_override("audio/driver/enable_input")
            .try_to::<bool>()
            .unwrap_or(false)
        {
            return;
        }
        let me = self.to_gd();
        let comms = globals::comms();
        comms.signals().on_adapter_changed().connect_other(
            &me,
            |this: &mut Self, enabled, _adapter| {
                this.voice_enabled = enabled;
                this.set_recording_enabled(false);
            },
        );
        let recorder = self.to_gd();
        recorder.signals().audio().connect_other(
            &comms,
            |comms: &mut crate::comms::communication_manager::CommunicationManager, frame| {
                comms.broadcast_voice(frame)
            },
        );
        self.setup_audio_server();
    }

    fn physics_process(&mut self, _dt: f64) {
        if !self.voice_enabled {
            return;
        }
        let held = Input::singleton().is_action_pressed(RECORD_ACTION);
        if self.is_recording != held {
            self.is_recording = held;
            ui_sounds::play_sound(
                if held {
                    "voice_chat_mic_on"
                } else {
                    "voice_chat_mic_off"
                },
                false,
                0.0,
            );
            self.set_recording_enabled(held);
        }
    }

    fn process(&mut self, _dt: f64) {
        if self.recording_enabled {
            let Some(effect_capture) = &mut self.effect_capture else {
                return;
            };

            let frames_available = effect_capture.get_frames_available();
            let stereo_data = effect_capture.get_buffer(frames_available);

            self.base_mut()
                .emit_signal("audio", &[stereo_data.to_variant()]);
        }
    }
}

#[godot_api]
impl VoiceChatRecorder {
    #[func]
    pub fn is_audio_server_ready(&self) -> bool {
        self.effect_capture.is_some()
    }

    #[func]
    pub fn setup_audio_server(&mut self) {
        let audio_server = AudioServer::singleton();
        let idx = audio_server.get_bus_index("Capture");
        if idx != -1 {
            let bus_effect: Option<Gd<AudioEffectCapture>> = {
                let mut found_effect = None;
                for i in 0..audio_server.get_bus_effect_count(idx) {
                    if let Some(bus_effect) = audio_server.get_bus_effect(idx, i) {
                        // Assuming you want to find the first `AudioEffectCapture`, so break after finding
                        found_effect = bus_effect.try_cast().ok();
                        if found_effect.is_some() {
                            break;
                        }
                    }
                }
                found_effect
            };

            self.effect_capture = bus_effect;
            self.base_mut().set_stream(&AudioStreamMicrophone::new_gd());
            self.base_mut().set_bus("Capture");
        }
    }

    #[signal]
    pub fn audio(frame: PackedVector2Array);

    #[func]
    pub fn set_recording_enabled(&mut self, enabled: bool) {
        if self.recording_enabled == enabled {
            return;
        }

        let Some(effect_capture) = &mut self.effect_capture else {
            return;
        };
        self.recording_enabled = enabled;

        if !enabled {
            effect_capture.clear_buffer();
            self.base_mut().stop();
        } else {
            // TODO: check if the order here matters
            effect_capture.clear_buffer();
            self.base_mut().play();
        }
    }
}
