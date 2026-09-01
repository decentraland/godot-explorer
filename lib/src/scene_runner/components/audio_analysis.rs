//! `PBAudioAnalysis` (1212) — spectrum and amplitude written back to the scene.
//!
//! Ported from unity-explorer's `native/audio-analysis` cdylib, with two
//! deliberate deviations, both permitted by the "same shape, same order of
//! magnitude" acceptance criterion:
//!
//! * Unity FFTs its interleaved stereo callback buffer as if it were mono at
//!   the output rate, which stretches the frequency axis by the channel count —
//!   what it labels 60 Hz is really 30 Hz. We downmix to mono first, so the band
//!   edges mean what they say.
//! * Unity's `bands_gain` default is calibrated for whatever buffer size its
//!   audio thread hands out, because `real_fft` neither windows nor normalizes.
//!   Our window is fixed, so absolute band values differ by a constant factor.
//!
//! Scope: `PBAudioSource` entities only. Unity also analyses its media player,
//! but our streams and videos are decoded by native platform players whose audio
//! never enters Godot, so there is no PCM to read (see #2800).

use godot::obj::Singleton;

use crate::{
    dcl::{
        crdt::SceneCrdtState,
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents},
    },
    scene_runner::scene::Scene,
};

const BANDS: usize = 8;
const DEFAULT_AMPLITUDE_GAIN: f32 = 5.0;
const DEFAULT_BANDS_GAIN: f32 = 0.05;
const MODE_LOGARITHMIC: i32 = 1;

/// Upper edge of each band in Hz; the last one is Nyquist.
const BAND_EDGES_HZ: [f32; BANDS - 1] = [60.0, 120.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0];

pub struct AnalysisResult {
    pub amplitude: f32,
    pub bands: [f32; BANDS],
}

fn largest_power_of_two(n: usize) -> usize {
    if n == 0 {
        0
    } else {
        1 << (usize::BITS - 1 - n.leading_zeros())
    }
}

fn rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|x| x * x).sum();
    (sum / samples.len() as f32).sqrt()
}

/// In-place iterative radix-2 FFT. Kept local instead of pulling in a DSP crate:
/// the whole DSP is under a hundred lines and the tree has no FFT dependency.
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two());

    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cur_r, mut cur_i) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (ar, ai) = (re[start + k], im[start + k]);
                let (br, bi) = (re[start + k + len / 2], im[start + k + len / 2]);
                let (tr, ti) = (br * cur_r - bi * cur_i, br * cur_i + bi * cur_r);
                re[start + k] = ar + tr;
                im[start + k] = ai + ti;
                re[start + k + len / 2] = ar - tr;
                im[start + k + len / 2] = ai - ti;
                let next_r = cur_r * wr - cur_i * wi;
                cur_i = cur_r * wi + cur_i * wr;
                cur_r = next_r;
            }
        }
        len <<= 1;
    }
}

fn freq_to_bin(freq: f32, nyquist: f32, bins: usize) -> usize {
    if nyquist <= 0.0 || bins < 2 {
        return 0;
    }
    let bin = (freq / nyquist * (bins as f32 - 1.0)).round();
    bin.clamp(0.0, bins as f32 - 1.0) as usize
}

/// `clamp(log10(gain * x + 1), 0, 1)` — Unity's `normalize_log`. Not dB, and
/// saturating at 1.0 once `x >= 9 / gain`.
fn normalize_log(x: f32, gain: f32) -> f32 {
    (x * gain + 1.0).log10().clamp(0.0, 1.0)
}

/// Mono samples in, amplitude + 8 bands out. `mode` follows `PBAudioAnalysisMode`.
pub fn analyze(
    samples: &[f32],
    sample_rate: f32,
    mode: i32,
    amplitude_gain: f32,
    bands_gain: f32,
) -> AnalysisResult {
    let zero = AnalysisResult {
        amplitude: 0.0,
        bands: [0.0; BANDS],
    };
    if samples.len() < 2 || sample_rate <= 0.0 {
        return zero;
    }

    let n = largest_power_of_two(samples.len().min(32768));
    if n < 2 {
        return zero;
    }
    // The tail: the caller hands us a rolling history, newest last.
    let window = &samples[samples.len() - n..];

    let amplitude_raw = rms(window);

    // Rectangular window, no zero padding — same as the reference.
    let mut re = window.to_vec();
    let mut im = vec![0.0f32; n];
    fft(&mut re, &mut im);

    let bins = n / 2 + 1;
    let spectrum: Vec<f32> = (0..bins)
        .map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt())
        .collect();

    let nyquist = sample_rate * 0.5;
    let mut bands_raw = [0.0f32; BANDS];
    for i in 0..BANDS {
        let start_freq = if i == 0 { 0.0 } else { BAND_EDGES_HZ[i - 1] };
        let end_freq = if i == BANDS - 1 {
            nyquist
        } else {
            BAND_EDGES_HZ[i]
        };
        let start_bin = freq_to_bin(start_freq, nyquist, bins);
        let end_bin = freq_to_bin(end_freq, nyquist, bins);

        let mut sum = 0.0;
        let mut count = 0usize;
        for bin in start_bin..end_bin {
            if bin < spectrum.len() {
                sum += spectrum[bin];
                count += 1;
            }
        }
        // The mean of the magnitudes, not the sum — matches the reference.
        bands_raw[i] = if count > 0 { sum / count as f32 } else { 0.0 };
    }

    if mode == MODE_LOGARITHMIC {
        let mut bands = [0.0f32; BANDS];
        for i in 0..BANDS {
            bands[i] = normalize_log(bands_raw[i], bands_gain);
        }
        AnalysisResult {
            amplitude: normalize_log(amplitude_raw, amplitude_gain),
            bands,
        }
    } else {
        AnalysisResult {
            amplitude: amplitude_raw,
            bands: bands_raw,
        }
    }
}

/// Analyse every entity that carries both `PBAudioAnalysis` and an `AudioSource`
/// node, and write the results back on the same LWW component.
pub fn update_audio_analysis(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    if scene.audio_sources.is_empty() {
        return;
    }

    let analysis_component = SceneCrdtStateProtoComponents::get_audio_analysis(crdt_state);
    let entities: Vec<_> = analysis_component
        .values
        .iter()
        .filter_map(|(entity, entry)| entry.value.as_ref().map(|value| (*entity, value.clone())))
        .filter(|(entity, _)| scene.audio_sources.contains_key(entity))
        .collect();

    if entities.is_empty() {
        return;
    }

    let sample_rate = godot::classes::AudioServer::singleton().get_mix_rate();
    let mut results = Vec::with_capacity(entities.len());

    for (entity, mut value) in entities {
        let Some(source) = scene.audio_sources.get_mut(&entity) else {
            continue;
        };

        let samples = source.bind_mut().read_analysis_samples(sample_rate);
        // No new audio this tick: Unity leaves the last values in place rather
        // than resetting them, so we skip the write too.
        if samples.is_empty() {
            continue;
        }

        let analysis = analyze(
            &samples,
            sample_rate,
            value.mode,
            value.amplitude_gain.unwrap_or(DEFAULT_AMPLITUDE_GAIN),
            value.bands_gain.unwrap_or(DEFAULT_BANDS_GAIN),
        );

        value.amplitude = analysis.amplitude;
        value.band_0 = analysis.bands[0];
        value.band_1 = analysis.bands[1];
        value.band_2 = analysis.bands[2];
        value.band_3 = analysis.bands[3];
        value.band_4 = analysis.bands[4];
        value.band_5 = analysis.bands[5];
        value.band_6 = analysis.bands[6];
        value.band_7 = analysis.bands[7];
        results.push((entity, value));
    }

    if results.is_empty() {
        return;
    }

    let analysis_component = SceneCrdtStateProtoComponents::get_audio_analysis_mut(crdt_state);
    for (entity, value) in results {
        analysis_component.put(entity, Some(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pure tone must land in the band that owns its frequency, and the RMS of
    /// a unit sine must be 1/sqrt(2). Covers the FFT, the band edges and the
    /// amplitude in one go.
    #[test]
    fn tone_lands_in_its_band() {
        let rate = 44100.0;
        let n = 2048;

        for (freq, expected_band) in [(40.0, 0), (200.0, 2), (3000.0, 6), (8000.0, 7)] {
            let samples: Vec<f32> = (0..n)
                .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin())
                .collect();

            let out = analyze(&samples, rate, 0, 1.0, 1.0);

            assert!(
                // A window that is not a whole number of cycles spreads the
                // RMS a little; 40 Hz only fits ~1.9 cycles in 2048 samples.
                (out.amplitude - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.03,
                "{freq} Hz: rms {} != 1/sqrt(2)",
                out.amplitude
            );

            let loudest = out
                .bands
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap();
            assert_eq!(loudest, expected_band, "{freq} Hz landed in band {loudest}");
        }
    }

    #[test]
    fn silence_and_degenerate_input_are_zero() {
        let out = analyze(&[0.0; 1024], 44100.0, 0, 1.0, 1.0);
        assert_eq!(out.amplitude, 0.0);
        assert!(out.bands.iter().all(|b| *b == 0.0));

        assert_eq!(analyze(&[], 44100.0, 0, 1.0, 1.0).amplitude, 0.0);
        assert_eq!(analyze(&[0.5; 1024], 0.0, 0, 1.0, 1.0).amplitude, 0.0);
    }

    /// MODE_LOGARITHMIC must stay inside [0, 1] whatever the input scale.
    #[test]
    fn logarithmic_mode_is_bounded() {
        let samples: Vec<f32> = (0..2048).map(|i| ((i % 7) as f32 - 3.0) * 50.0).collect();
        let out = analyze(&samples, 44100.0, MODE_LOGARITHMIC, 5.0, 0.05);
        assert!((0.0..=1.0).contains(&out.amplitude));
        assert!(out.bands.iter().all(|b| (0.0..=1.0).contains(b)));
    }
}
