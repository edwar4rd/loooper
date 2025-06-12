use crate::filter::{Delay, Distortion, Filter, Wah};
use std::sync::Arc;

pub struct AudioCallbackSettings {
    pub sample_rate: usize,
    pub in_port: jack::Port<jack::AudioIn>,
    pub out_port: jack::Port<jack::AudioOut>,
    pub enabled: Arc<std::sync::atomic::AtomicBool>,
    pub countin: Arc<std::sync::atomic::AtomicBool>,
    pub countin_length: Arc<std::sync::atomic::AtomicU32>,
    pub rolling_tx: tokio::sync::mpsc::UnboundedSender<()>,
    pub mbpm: Arc<std::sync::atomic::AtomicU32>,
    pub loop_length: Vec<Arc<std::sync::atomic::AtomicU32>>,
    pub loop_starting: Vec<Arc<std::sync::atomic::AtomicBool>>,
    pub loop_playing: Vec<Arc<std::sync::atomic::AtomicBool>>,
    pub loop_recording: Vec<Arc<std::sync::atomic::AtomicBool>>,
    pub current_millibeat: Arc<std::sync::atomic::AtomicU32>,
    pub drum_index: Arc<std::sync::atomic::AtomicU32>,
}

pub fn create_callback(settings: AudioCallbackSettings) -> impl jack::ProcessHandler {
    let AudioCallbackSettings {
        sample_rate,
        in_port,
        mut out_port,
        enabled,
        countin,
        countin_length,
        rolling_tx,
        mbpm,
        loop_length,
        loop_starting,
        loop_playing,
        loop_recording,
        current_millibeat,
        drum_index,
    } = settings;

    let mut audio_clock: u64 = 0; // using u32 should panic in about a day
    let enabled_clone = enabled.clone();
    let mut last_enabled = false;
    let countin_clone = countin.clone();
    let mut countin_started = false;
    let countin_length_clone = countin_length.clone();
    let mut countin_left = 0;
    let mut rolling = false;
    let mbpm_clone = mbpm.clone();
    let mut adsr = super::adsr::ADSR::new(0.01, 0.1, 0.2, 0.02);
    let mut click_vol = 0.2;
    let mut click_osc = super::oscillator::Oscillator::new(523.25 / 2.0, sample_rate);
    let mut last_beat_pos = 0.999;
    let current_millibeat_clone = current_millibeat.clone();
    let mut current_beat = 0; // Which milli beat we're in, start at beat 1.0 -> 1000, including the count-in
    let drum_index_clone = drum_index.clone();
    // let tx_clone = tx.clone();
    let mut loop_buffers = (0..8)
        .map(|_| {
            let mut buf_vec = Vec::<f32>::with_capacity(sample_rate * 2 * 33);
            buf_vec.resize(sample_rate * 2 * 33, 0.0);
            buf_vec.into_boxed_slice()
        })
        .collect::<Vec<_>>();
    let mut loop_filled = [false; 8];
    let mut loop_looping = [false; 8];
    let mut loop_capturing = [false; 8];
    let mut loop_pos = [0usize; 8];
    let loop_length_clone = loop_length.clone();
    let loop_starting_clone = loop_starting.clone();
    let loop_playing_clone = loop_playing.clone();
    let loop_recording_clone = loop_recording.clone();
    let mut loop_recording_start_beat = [0; 8];

    const DELAY_MS: usize = 250;
    const FEEDBACK: f32 = 0.4;
    const WET: f32 = 0.8;
    let delay_samples = (sample_rate * DELAY_MS) / 1000;
    let mut monitor_delay = Delay::new(delay_samples, FEEDBACK, WET);
    let mut playback_delay = vec![Delay::new(delay_samples, FEEDBACK, WET); 8];
    let mut distortion = Distortion::new(8.0, 0.5);
    let _wah = Wah::new(
        sample_rate as f32,
        2.0,    // sweep at 2 Hz
        500.0,  // min 500 Hz
        3000.0, // max 3 kHz
        0.8,
    ); // resonance

    let callback_closure = move |_client: &jack::Client, ps: &jack::ProcessScope| {
        let sample_rate = sample_rate as u64;
        let in_port = in_port.as_slice(ps);
        let out_port = out_port.as_mut_slice(ps);

        // We're not enabled, output nothing and quit callback
        if !enabled_clone.load(std::sync::atomic::Ordering::Relaxed) {
            out_port.fill(0.0);
            last_enabled = false;
            return jack::Control::Continue;
        }

        if !last_enabled {
            // We just got enabled, reset relavent audio callback states
            audio_clock = 0;
            click_osc.set_freq(523.25 / 2.0);
        }
        last_enabled = true;

        // Get bpm * 1000 from the gui thread (this is currently only altered during SetUp -> Prepare)
        let mbpm = mbpm_clone.load(std::sync::atomic::Ordering::Relaxed);
        let mspb = (60.0 / mbpm as f32 * 1000.0 * 1000.0) as u64;

        let drum_index = drum_index_clone.load(std::sync::atomic::Ordering::Relaxed);

        let mut countin_local = countin_clone.load(std::sync::atomic::Ordering::Relaxed);

        for (in_sample, out_sample) in in_port.iter().zip(out_port.iter_mut()) {
            // Where we are inside a beat (0.0 - 1.0)
            let beat_pos = (audio_clock % (sample_rate * mspb / 1000)) as f32
                / ((sample_rate * mspb / 1000) as f32);
            let current_subbeat = (beat_pos * 1000.0) as u32;

            // Set the sample to the input sample (monitoring)
            let temp_sample = distortion.apply(*in_sample);
            *out_sample = monitor_delay.apply(temp_sample);

            // We entered a new beat
            if beat_pos < last_beat_pos {
                // Check if Count-in just started
                // This should only happen once per callback
                if !countin_started && countin_local {
                    // Reset the audio clock
                    audio_clock = 0;
                    // Reset the beat counters
                    current_beat = 0;
                    current_millibeat_clone.store(1000, std::sync::atomic::Ordering::Relaxed);

                    // Set up the countin flags
                    countin_left = countin_length_clone.load(std::sync::atomic::Ordering::Relaxed);
                    countin_started = true;
                    countin_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                    countin_local = false;
                }

                // Increase our beat counter
                current_beat += 1;

                // Reset the adsr for metronome
                adsr.reset();

                // We change the metronome volume and frequency for different phases
                let click_freq = if countin_started {
                    if countin_left == 0 {
                        countin_started = false;
                        let _ = rolling_tx.send(());
                        rolling = true;
                        current_beat = 1;
                        523.25
                    } else {
                        countin_left -= 1;
                        if countin_left % 4 == 3 {
                            523.25
                        } else {
                            523.25 / 2.0
                        }
                    }
                } else if rolling {
                    if current_beat % 4 == 1 {
                        523.25
                    } else {
                        523.25 / 2.0
                    }
                } else {
                    440.0
                };
                click_osc.set_freq(click_freq);
                click_vol = if rolling {
                    0.05
                } else if countin_started {
                    0.4
                } else {
                    0.2
                };

                if rolling {
                    // Set up loop recording and playback states
                    for index in 0..8 {
                        let length =
                            loop_length_clone[index].load(std::sync::atomic::Ordering::Relaxed);
                        if !match length {
                            0 => false,
                            1 => true,
                            2 => current_beat % 2 == 1,
                            3..=4 => current_beat % 4 == 1,
                            5..=8 => current_beat % 8 == 1,
                            9..=16 => current_beat % 16 == 1,
                            17..=32 => current_beat % 32 == 1,
                            _ => current_beat == 1,
                        } {
                            continue;
                        }

                        if loop_capturing[index]
                            && (current_beat - loop_recording_start_beat[index]) >= length
                        {
                            // recording ended, start looping
                            loop_filled[index] = true;
                            loop_capturing[index] = false;
                            loop_looping[index] = true;
                            loop_recording_clone[index]
                                .store(false, std::sync::atomic::Ordering::Relaxed);
                        }

                        if loop_starting_clone[index].load(std::sync::atomic::Ordering::Relaxed) {
                            if loop_filled[index] {
                                loop_looping[index] = true;
                            } else {
                                loop_capturing[index] = true;
                                loop_recording_start_beat[index] = current_beat;
                                loop_pos[index] = 0;
                                loop_recording_clone[index]
                                    .store(true, std::sync::atomic::Ordering::Relaxed);
                            }
                        } else if loop_filled[index] {
                            loop_looping[index] = false;
                        }

                        if loop_looping[index] {
                            loop_pos[index] = 0;
                            loop_playing_clone[index]
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                        } else {
                            loop_playing_clone[index]
                                .store(false, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            }
            last_beat_pos = beat_pos;

            current_millibeat_clone.store(
                current_beat * 1000 + current_subbeat,
                std::sync::atomic::Ordering::Relaxed,
            );

            {
                // Set the adsr to release state after half a beat
                if beat_pos > 0.25 {
                    adsr.release();
                }
                let vol = adsr.forward(1.0 / (sample_rate as f32));

                let wave = click_osc.increment() * click_vol;
                let amp = vol * wave;
                *out_sample += amp;
            }

            for index in 0..8 {
                if loop_looping[index] {
                    let dry_sample = loop_buffers[index][loop_pos[index]];
                    let wet_sample = playback_delay[index].apply(dry_sample);
                    *out_sample += wet_sample;
                }

                if loop_capturing[index] {
                    let original_sample = *in_sample;
                    let distortion_sample = distortion.apply(original_sample);
                    //let wah_sample = wah.apply(distortion_sample);
                    loop_buffers[index][loop_pos[index]] = distortion_sample;
                }

                if loop_looping[index] || loop_capturing[index] {
                    loop_pos[index] += 1;
                }
            }

            audio_clock += 1;
        }
        jack::Control::Continue
    };

    jack::contrib::ClosureProcessHandler::new(callback_closure)
}
