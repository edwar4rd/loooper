use color_eyre::{Result, eyre::eyre};
use cpal::traits::{DeviceTrait, HostTrait};

pub fn host_device_setup() -> Result<(
    cpal::Host,
    cpal::SupportedStreamConfig,
    cpal::Device,
    cpal::Device,
)> {
    let host = cpal::host_from_id(cpal::HostId::Jack).unwrap_or_else(|_| {
        println!("Jack/Pipewire host not available, falling back to default host");
        cpal::default_host()
    });

    let in_device = host
        .default_input_device()
        .ok_or_else(|| eyre!("Default input device is not available"))?;
    println!("Input device : {}", in_device.name()?);

    let in_config = in_device.default_input_config()?;
    println!("Input config : {:?}", in_config);

    let out_device = host
        .default_output_device()
        .ok_or_else(|| eyre!("Default output device is not available"))?;
    println!("Output device : {}", out_device.name()?);

    let out_config = out_device
        .supported_output_configs()?
        .find(|config| {
            config.channels() == in_config.channels()
                && config.sample_format() == in_config.sample_format()
                && config
                    .try_with_sample_rate(in_config.sample_rate())
                    .is_some()
        })
        .map(|range| range.with_sample_rate(in_config.sample_rate()))
        .ok_or_else(|| eyre!("Output device doesn't support the same format as the input"))?;

    if let cpal::SupportedBufferSize::Unknown = in_config.buffer_size() {
        return Err(eyre!("Input device doesn't support a fixed buffer size"));
    }
    assert_eq!(in_config, out_config);

    println!("Output config : {:?}", out_config);

    Ok((host, in_config, in_device, out_device))
}

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32},
};
use tokio::sync::mpsc;

#[test]
fn test_host_device_setup() {
    let result = host_device_setup();
    assert!(result.is_ok());
    let _ = result.unwrap();
}

pub fn create_audio_streams(
    _host: cpal::Host,
    supported_config: cpal::SupportedStreamConfig,
    input_device: cpal::Device,
    output_device: cpal::Device,
) -> Result<(cpal::Stream, cpal::Stream, AudioState)> {
    use ringbuf::{
        HeapRb,
        traits::{Consumer, Producer, Split},
    };

    let enabled = Arc::new(AtomicBool::new(false));
    let mbpm = Arc::new(AtomicU32::new(120));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let state = AudioState {
        enabled: enabled.clone(),
        mbpm: mbpm.clone(),
        errors: rx,
    };

    let (buffer_size, config) = {
        let mut config = supported_config.config();
        if let cpal::SupportedBufferSize::Range { min, max: _ } = supported_config.buffer_size() {
            config.buffer_size = cpal::BufferSize::Fixed(*min);
            (*min, config)
        } else {
            unreachable!("Buffer size is not fixed");
        }
    };

    // Create a delay in case the input and output devices aren't synced.
    // let latency_frames = (1000.0 / 1_000.0) * config.sample_rate().0 as f32;
    // let latency_samples = latency_frames as usize * config.channels() as usize;
    let latency_samples = buffer_size as usize * supported_config.channels() as usize;

    // The buffer to share samples
    let ring = HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    // Fill the samples with 0.0 equal to the length of the delay.
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.try_push(0.0).unwrap();
    }

    let enabled_clone = enabled.clone();
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        if !enabled_clone.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let mut output_fell_behind = false;
        for &sample in data {
            if producer.try_push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            // eprintln!("output stream fell behind: try increasing latency");
        }
    };

    fn err_fn(err: cpal::StreamError) {
        // eprintln!("an error occurred on stream: {}", err);
    }

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        if !enabled.load(std::sync::atomic::Ordering::Relaxed) {
            for sample in data {
                *sample = 0.0;
            }
            return;
        }

        let mut input_fell_behind = false;
        for sample in data {
            *sample = match consumer.try_pop() {
                Some(s) => s,
                None => {
                    input_fell_behind = true;
                    0.0
                }
            };
        }
        if input_fell_behind {
            // eprintln!("input stream fell behind: try increasing latency");
        }
    };

    let input_stream =
        input_device.build_input_stream(&supported_config.config(), input_data_fn, err_fn, None)?;
    let output_stream = output_device.build_output_stream(
        &supported_config.config(),
        output_data_fn,
        err_fn,
        None,
    )?;
    println!("Successfully built streams.");
    Ok((input_stream, output_stream, state))
}

#[derive(Debug)]
pub struct AudioState {
    pub enabled: Arc<AtomicBool>,
    pub mbpm: Arc<AtomicU32>,
    pub errors: mpsc::UnboundedReceiver<String>,
}
