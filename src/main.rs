use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use std::error::Error;
use std::process::Command;

const VIRTUAL_SINK: &str = "AudioBloatware_Virtual";

fn try_setup_virtual_sink() {
    let sink_exists = Command::new("pactl")
        .args(["list", "short", "sinks"])
        .output()
        .ok()
        .map(|out| {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .any(|line| line.split_whitespace().nth(1) == Some(VIRTUAL_SINK))
        })
        .unwrap_or(false);

    if sink_exists {
        return;
    }

    let sink_name_arg = format!("sink_name={VIRTUAL_SINK}");

    match Command::new("pactl")
        .args([
            "load-module",
            "module-null-sink",
            sink_name_arg.as_str(),
            "sink_properties=device.description=AudioBloatware_Input",
        ])
        .output()
    {
        Ok(out) if out.status.success() => {
            let _ = Command::new("pactl")
                .args(["set-default-sink", VIRTUAL_SINK])
                .output();
        }
        Ok(out) => {
            eprintln!(
                "Warning: failed to create virtual sink: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(err) => {
            eprintln!("Warning: pactl not available or failed: {err}");
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("======================================");
    println!("AUDIOBLOATWARE - UNIVERSAL BY ZHAR");
    println!("======================================");

    try_setup_virtual_sink();

    let host = cpal::default_host();

    let input_device = host
        .default_input_device()
        .ok_or("No input device available.")?;
    let output_device = host
        .default_output_device()
        .ok_or("No output device available.")?;

    let input_supported = input_device.default_input_config()?;
    let output_supported = output_device.default_output_config()?;

    if input_supported.sample_format() != cpal::SampleFormat::F32
        || output_supported.sample_format() != cpal::SampleFormat::F32
    {
        return Err("This program requires f32 audio format.".into());
    }

    let input_config: cpal::StreamConfig = input_supported.into();
    let output_config: cpal::StreamConfig = output_supported.into();

    let input_channels = input_config.channels as usize;
    let output_channels = output_config.channels as usize;

    if input_channels == 0 || output_channels == 0 {
        return Err("Invalid channel configuration.".into());
    }

    if input_config.sample_rate != output_config.sample_rate {
        return Err(format!(
            "Sample rates do not match: input {} Hz, output {} Hz.",
            input_config.sample_rate.0, output_config.sample_rate.0
        )
        .into());
    }

    let rb = HeapRb::<f32>::new((input_config.sample_rate.0 as usize).saturating_mul(4));
    let (mut prod, mut cons) = rb.split();

    let input_stream = input_device.build_input_stream(
        &input_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for frame in data.chunks(input_channels) {
                if input_channels == 1 {
                    let s = frame[0];
                    let _ = prod.push(s);
                    let _ = prod.push(s);
                } else {
                    let l = frame[0];
                    let r = frame[1];
                    let _ = prod.push(l);
                    let _ = prod.push(r);
                }
            }
        },
        |err| eprintln!("Input stream error: {}", err),
        None,
    )?;

    let mut envelope = 0.0f32;
    let threshold_gate = 0.0012;

    let comp_threshold = 0.25;
    let comp_ratio = 3.0;
    let makeup_gain = 1.8;
    let mut current_gain = 1.0f32;

    let mut bass_state_l = 0.0f32;
    let mut bass_state_r = 0.0f32;
    let crossover_freq = 0.02;
    let boost_bass = 1.15;
    let boost_mid_high = 1.25;

    let side_boost = 1.20;

    let output_stream = output_device.build_output_stream(
        &output_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(output_channels) {
                let in_l = cons.pop().unwrap_or(0.0);
                let in_r = cons.pop().unwrap_or(0.0);

                let mono_mix = (in_l.abs() + in_r.abs()) * 0.5;
                envelope = envelope * 0.99 + mono_mix * 0.01;

                let gate_gain = if envelope < threshold_gate {
                    (envelope / threshold_gate).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                let gated_l = in_l * gate_gain;
                let gated_r = in_r * gate_gain;

                bass_state_l = crossover_freq * gated_l + (1.0 - crossover_freq) * bass_state_l;
                bass_state_r = crossover_freq * gated_r + (1.0 - crossover_freq) * bass_state_r;

                let mid_highs_l = gated_l - bass_state_l;
                let mid_highs_r = gated_r - bass_state_r;

                let eq_l = (bass_state_l * boost_bass) + (mid_highs_l * boost_mid_high);
                let eq_r = (bass_state_r * boost_bass) + (mid_highs_r * boost_mid_high);

                let abs_eq_max = eq_l.abs().max(eq_r.abs());
                let desired_gain = if abs_eq_max > comp_threshold {
                    let target_level = comp_threshold + (abs_eq_max - comp_threshold) / comp_ratio;
                    (target_level / abs_eq_max).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                let alpha_comp = if desired_gain < current_gain {
                    0.01
                } else {
                    0.001
                };
                current_gain = alpha_comp * desired_gain + (1.0 - alpha_comp) * current_gain;

                let comp_l = eq_l * current_gain * makeup_gain;
                let comp_r = eq_r * current_gain * makeup_gain;

                let mid = (comp_l + comp_r) * 0.5;
                let side = (comp_l - comp_r) * 0.5;

                let final_l = (mid + (side * side_boost)).clamp(-1.0, 1.0);
                let final_r = (mid - (side * side_boost)).clamp(-1.0, 1.0);
                let final_mono = ((final_l + final_r) * 0.5).clamp(-1.0, 1.0);

                if output_channels == 1 {
                    frame[0] = final_mono;
                } else {
                    frame[0] = final_l;
                    frame[1] = final_r;

                    for ch in 2..output_channels {
                        frame[ch] = final_mono;
                    }
                }
            }
        },
        |err| eprintln!("Output stream error: {}", err),
        None,
    )?;

    input_stream.play()?;
    output_stream.play()?;

    println!("-> Engines active: Soft Gate, 2-Band EQ, Stereo Compressor, Mid/Side Spatializer");
    std::thread::park();

    Ok(())
}
