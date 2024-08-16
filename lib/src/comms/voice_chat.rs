use godot::{
    engine::{AudioEffectCapture, AudioServer, AudioStreamMicrophone},
    prelude::*,
};

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
struct VoiceChatRecorder {
    recording_enabled: bool,
    effect_capture: Option<Gd<AudioEffectCapture>>,

    #[base]
    base: Base<AudioStreamPlayer>,
}

#[godot_api]
impl IAudioStreamPlayer for VoiceChatRecorder {
    fn process(&mut self, _dt: f64) {
        if self.recording_enabled {
            let Some(effect_capture) = &mut self.effect_capture else {
                return;
            };

            let frames_available = effect_capture.get_frames_available();
            let stereo_data = effect_capture.get_buffer(frames_available);

            self.base
                .emit_signal("audio".into(), &[stereo_data.to_variant()]);
        }
    }
}

#[godot_api]
impl VoiceChatRecorder {
    #[func]
    fn is_audio_server_ready(&self) -> bool {
        self.effect_capture.is_some()
    }

    #[func]
    fn setup_audio_server(&mut self) {
        let mut audio_server = AudioServer::singleton();
        let idx = audio_server.get_bus_index("Capture".into());
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
            self.base.set_stream(AudioStreamMicrophone::new().upcast());
            self.base.set_bus("Capture".into());
        }
    }

    #[signal]
    fn audio(frame: PackedVector2Array);

    #[func]
    fn set_recording_enabled(&mut self, enabled: bool) {
        if self.recording_enabled == enabled {
            return;
        }

        let Some(effect_capture) = &mut self.effect_capture else {
            return;
        };
        self.recording_enabled = enabled;

        if !enabled {
            effect_capture.clear_buffer();
            self.base.stop();
        } else {
            self.base.play();
            effect_capture.clear_buffer();
        }
    }
}
