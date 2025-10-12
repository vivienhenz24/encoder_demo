use hound::{WavReader, WavWriter};
use realfft::RealFftPlanner;
use std::fs;
use std::path::Path;

// =============================================================================
// CONSTANTS - Watermark configuration
// =============================================================================

// Pilot pattern: A known sequence at the start to help decoder find the threshold
// Alternating 0s and 1s give us clear separation between high and low magnitudes
pub const PILOT_PATTERN: [u8; 8] = [0, 1, 0, 1, 0, 1, 0, 1];

// Duration of each audio frame we process (in seconds)
// 20ms is standard for speech processing - short enough to be locally stationary
pub const WATERMARK_FRAME_DURATION: f32 = 0.02;

// Sample normalization divisor for i16 -> f32 conversion
const SAMPLE_DIVISOR: f32 = 32768.0;

// Quantization multiplier for f32 -> i16 conversion
const SAMPLE_MULTIPLIER: f32 = 32767.0;

// Starting frequency bin index (skip low frequencies)
const START_BIN: usize = 10;

// Watermark embedding strength (magnitude scaling factor)
const WATERMARK_STRENGTH: f32 = 0.15;

// Input and output file paths
const INPUT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/input_data/OSR_us_000_0057_8k.wav"
);
const OUTPUT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/output_data/OSR_us_000_0057_8k_watermarked.wav"
);

// =============================================================================
// ORCHESTRATOR: Main entry point that coordinates the encoding pipeline
// =============================================================================

pub fn encode_sample(message: &str) {
    // Step 1: Load audio and get normalized samples + metadata
    let (normalized, spec) = load_and_normalize_audio(Path::new(INPUT_PATH));

    // Step 2: Build the bit sequence (pilot + length + message)
    let bits = build_bit_sequence(message);

    // Step 3: Embed bits into audio via FFT processing
    let encoded = embed_watermark_fft(&normalized, &bits, spec.sample_rate);

    // Step 4: Convert back to i16 samples
    let quantized = quantize_to_i16(encoded);

    // Step 5: Write the watermarked audio to disk
    write_wav_file(Path::new(OUTPUT_PATH), &quantized, spec);
}

// =============================================================================
// STEP 1: Load and normalize audio
// =============================================================================

fn load_and_normalize_audio(input_path: &Path) -> (Vec<f32>, hound::WavSpec) {
    println!("Loading clean audio from {}", input_path.display());

    let mut reader = WavReader::open(input_path).expect("failed to open wav file");


    // Read and normalize samples in a single pass: i16 -> f32 in [-1.0, 1.0]
    let mut normalized: Vec<f32> = Vec::new();

    for sample_result in reader.samples::<i16>() {
       
        let sample = sample_result.expect("failed to open sound file");
        let normalized_sample = (sample as f32) / SAMPLE_DIVISOR;

        normalized.push(normalized_sample);

    }

    let spec = reader.spec();
    
    println!(
        "Read and normalized {} samples at {} Hz",
        normalized.len(),
        spec.sample_rate
    );

    (normalized, spec)
}

// =============================================================================
// STEP 2: Build bit sequence (pilot + length + message)
// =============================================================================

fn build_bit_sequence(message: &str) -> Vec<u8> {
    let message_bytes = message.as_bytes();
    let length_header = message_bytes.len() as u16;

    let mut bits = Vec::new();

    // 1. Pilot pattern for threshold calibration
    bits.extend_from_slice(&PILOT_PATTERN);

    // 2. Length header (16 bits, MSB first)
    for shift in (0..16).rev() {
        bits.push(((length_header >> shift) & 1) as u8);
    }

    // Position:  15 14 13 12 11 10  9  8  7  6  5  4  3  2  1  0
    // Binary:     0  0  0  0  0  0  0  0  0  0  0  0  0  1  0  1

    // 3. Message payload (8 bits per byte, MSB first)
    for &byte in message_bytes {
        for shift in (0..8).rev() {
            bits.push((byte >> shift) & 1);
        }
    }

    println!(
        "Encoding message {:?} ({} bytes)",
        message,
        message_bytes.len()
    );
    println!(
        "Total bits to embed (pilot + length + data): {}",
        bits.len()
    );

    bits
}

// =============================================================================
// STEP 3: Embed watermark using FFT
// =============================================================================

fn embed_watermark_fft(normalized: &[f32], bits: &[u8], sample_rate: u32) -> Vec<f32> {
    // Calculate frame parameters
    let frame_len = ((sample_rate as f32 * WATERMARK_FRAME_DURATION)
        .round()
        .max(1.0)) as usize;
    let fft_len = frame_len.next_power_of_two().max(2);

    // Setup FFT planner and plans
    let mut planner = RealFftPlanner::<f32>::new();
    let forward = planner.plan_fft_forward(fft_len);
    let inverse = planner.plan_fft_inverse(fft_len);

    // Allocate reusable buffers
    let mut scratch_forward = forward.make_scratch_vec();
    let mut scratch_inverse = inverse.make_scratch_vec();
    let mut buffer = vec![0.0f32; fft_len];
    let mut spectrum = forward.make_output_vec();
    let mut reconstructed = inverse.make_output_vec();

    let mut encoded = Vec::with_capacity(normalized.len());
    let mut frame_count = 0;

    // Process each frame
    let mut offset = 0;
    while offset < normalized.len() {
        let end = (offset + frame_len).min(normalized.len());
        let chunk = &normalized[offset..end];

        // Prepare FFT input
        buffer.fill(0.0);
        buffer[..chunk.len()].copy_from_slice(chunk);

        // Forward FFT: time domain -> frequency domain
        forward
            .process_with_scratch(&mut buffer, &mut spectrum, &mut scratch_forward)
            .expect("FFT failed");

        // Embed bits by scaling frequency bins
        let usable = spectrum.len().saturating_sub(START_BIN);
        let bits_to_encode = bits.len().min(usable);

        for bit_idx in 0..bits_to_encode {
            let bin = START_BIN + bit_idx;
            let bit = bits[bit_idx];
            let scale = if bit == 1 {
                1.0 + WATERMARK_STRENGTH
            } else {
                1.0 - WATERMARK_STRENGTH
            };

            spectrum[bin].re *= scale;
            spectrum[bin].im *= scale;
        }

        // Inverse FFT: frequency domain -> time domain
        inverse
            .process_with_scratch(&mut spectrum, &mut reconstructed, &mut scratch_inverse)
            .expect("IFFT failed");

        // Normalize and append to output
        let fft_scale = fft_len as f32;
        encoded.extend(reconstructed[..chunk.len()].iter().map(|x| x / fft_scale));

        offset += chunk.len();
        frame_count += 1;
    }

    println!(
        "Processed {} frames ({} samples per frame, FFT len {})",
        frame_count, frame_len, fft_len
    );

    encoded
}

// =============================================================================
// STEP 4: Quantize to i16 samples
// =============================================================================

fn quantize_to_i16(encoded: Vec<f32>) -> Vec<i16> {
    let quantized: Vec<i16> = encoded
        .into_iter()
        .map(|sample| {
            let scaled = (sample.clamp(-1.0, 1.0) * SAMPLE_MULTIPLIER).round();
            scaled.clamp(i16::MIN as f32, i16::MAX as f32) as i16
        })
        .collect();

    println!("Converted floating point samples back to i16");
    quantized
}

// =============================================================================
// STEP 5: Write WAV file to disk
// =============================================================================

fn write_wav_file(output_path: &Path, quantized: &[i16], spec: hound::WavSpec) {
    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("failed to create output directory");
    }

    // Create WAV writer and write samples
    let mut writer = WavWriter::create(output_path, spec).expect("failed to create wav writer");
    for &sample in quantized {
        writer.write_sample(sample).expect("failed to write sample");
    }
    writer.finalize().expect("failed to finalize wav file");

    println!("Wrote watermarked audio to {}", output_path.display());
}
