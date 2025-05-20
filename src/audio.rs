use color_eyre::{
    Result,
    eyre::eyre,
};
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

    assert_eq!(in_config, out_config);

    println!("Output config : {:?}", out_config);

    Ok((host, in_config, in_device, out_device))
}

#[test]
fn test_host_device_setup() {
    let result = host_device_setup();
    assert!(result.is_ok());
    let _ = result.unwrap();
}
