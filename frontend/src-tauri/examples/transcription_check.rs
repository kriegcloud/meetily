//! Check ALSA device resolution and local Whisper inference without changing meeting data.
//! Input audio must be mono, 16 kHz, little-endian f32 PCM (convert with FFmpeg).
use anyhow::{ensure, Context, Result};
use app_lib::audio::devices::{get_device_and_config, parse_audio_device};
use app_lib::whisper_engine::whisper_engine::WhisperEngine;
use cpal::traits::DeviceTrait;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 3,
        "usage: transcription_check MODELS_DIR AUDIO.f32"
    );
    #[cfg(target_os = "linux")]
    for name in ["pulse (input)", "meetily_monitor (System Audio) (output)"] {
        let device = parse_audio_device(name)?;
        let (resolved, config) = get_device_and_config(&device).await?;
        println!("Resolved {name} -> {}: {config:?}", resolved.name()?);
    }
    let bytes = std::fs::read(&args[2]).context("read 16 kHz mono f32 PCM")?;
    ensure!(bytes.len() % 4 == 0, "partial f32 sample");
    let samples: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|s| f32::from_le_bytes(s.try_into().unwrap()))
        .collect();
    ensure!(!samples.is_empty(), "empty audio input");
    let segments = app_lib::audio::vad::get_speech_chunks(&samples, 500)?;
    ensure!(
        !segments.is_empty(),
        "voice activity detector returned no speech segments"
    );
    println!(
        "Voice activity detector produced {} speech segments",
        segments.len()
    );
    let engine = WhisperEngine::new_with_models_dir(Some(PathBuf::from(&args[1])))?;
    engine.discover_models().await?;
    engine.load_model("large-v3").await?;
    let text = engine.transcribe_audio(samples, Some("en".into())).await?;
    ensure!(
        !text.trim().is_empty(),
        "Whisper returned an empty transcript"
    );
    println!("Transcript: {text}");
    engine.unload_model().await;
    Ok(())
}
