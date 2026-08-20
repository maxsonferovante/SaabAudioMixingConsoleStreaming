use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use ringbuf::traits::Consumer;
use ringbuf::HeapCons;
use tracing::info;

pub struct CpalAudioPlayback {
    _stream: cpal::Stream,
}

impl CpalAudioPlayback {
    pub fn start(mut consumer: HeapCons<f32>) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No default output audio device found"))?;

        let default_config = device.default_output_config()?;
        info!(
            "Default output device: {}, format: {:?}, sample rate: {}",
            device.name().unwrap_or_default(),
            default_config.sample_format(),
            default_config.sample_rate().0
        );

        let config: StreamConfig = default_config.into();
        let channels = config.channels as usize;

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    for sample in frame.iter_mut() {
                        *sample = consumer.try_pop().unwrap_or(0.0);
                    }
                }
            },
            move |err| {
                tracing::error!("CPAL audio playback error: {:?}", err);
            },
            None,
        )?;

        stream.play()?;
        Ok(Self { _stream: stream })
    }
}
