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

    fn ready(&mut self) {
        let mut audio_server = AudioServer::singleton();
        let idx = audio_server.get_bus_index("Capture".into());
        if idx != -1 {
            let Some(bus_effect) = audio_server.get_bus_effect(idx, 0) else {
                return;
            };

            self.effect_capture = bus_effect.try_cast().ok();
            self.base.set_stream(AudioStreamMicrophone::new().upcast());
            self.base.set_bus("Capture".into());
        }
    }
}

#[godot_api]
impl VoiceChatRecorder {
    #[signal]
    fn audio(enabled: bool);

    #[func]
    fn set_recording_enabled(&mut self, enabled: bool) {
        if self.recording_enabled == enabled {
            return;
        }

        self.recording_enabled = enabled;
        let Some(effect_capture) = &mut self.effect_capture else {
            return;
        };

        if enabled {
            effect_capture.clear_buffer();
            self.base.stop();
        } else {
            self.base.play();
            effect_capture.clear_buffer();
        }
    }
}
