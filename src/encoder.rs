use hound::{WavReader, WavWriter};
use realfft::RealFftPlanner;
use std::path::Path;

// =============================================================================
// CONSTANTS - Watermark configuration
// =============================================================================

// Pilot pattern: A known sequence at the start to help decoder find the threshold
// Alternating 0s and 1s give us clear separation between high and low magnitudes
pub const PILOT_PATTERN: [u8; 8] = [0, 1, 0, 1, 0, 1, 0, 1];



// Sample normalization divisor for i16 -> f32 conversion
const SAMPLE_DIVISOR: f32 = 32768.0;



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
    let encoded = embed_watermark_fft(&normalized, &bits);

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

fn embed_watermark_fft(audio: &[f32], bits: &[u8]) -> Vec<f32> {
    let frame_len = (8000.0 * 0.032) as usize;  // 8000 Hz * 32ms = 256 samples

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(frame_len);
    let ifft = planner.plan_fft_inverse(frame_len);

    let mut buffer = vec![0.0f32; frame_len];

    //buffer (256 slots):
    //[___|___|___|___|___| ... |___|___|___]

    let mut spectrum = fft.make_output_vec();
    let mut output = Vec::new();

    // Process each frame
    for chunk in audio.chunks(frame_len) {
        // Load audio
        buffer[..frame_len].fill(0.0); //wipe clean every time becasue multiple iterations
        buffer[..chunk.len()].copy_from_slice(chunk); //copies chunk into our empty slots

        // Time → Frequency
        fft.process(&mut buffer, &mut spectrum).expect("FFT failed"); //i will explain in the decoder video

        // Embed bits: boost (1.15) or reduce (0.85) frequency amplitudes
        // Produces: &0, &1, &0, &1, &0, &1, ...
        // (references to each bit)
        // Same as spectrum[10..129]
        // Includes: spectrum[10], spectrum[11], spectrum[12], ..., spectrum[128]
        // That's 119 elements

        //  Left side:     Right side:
        // &0     ←──→  bin10
        // &1     ←──→  bin11
        // &0     ←──→  bin12
        // &1     ←──→  bin13
         // ...
        for (&bit, bin) in bits.iter().zip(&mut spectrum[10..]) {
            let scale = if bit == 1 { 2.0 } else { 0.0 };
            bin.re *= scale;
            bin.im *= scale;
        }

        // Frequency → Time
        ifft.process(&mut spectrum, &mut buffer).expect("IFFT failed");

        // Normalize and append
        output.extend(buffer[..chunk.len()].iter().map(|x| x / frame_len as f32));
    }

    output
}

// =============================================================================
// STEP 4: Quantize to i16 samples
// =============================================================================

fn quantize_to_i16(encoded: Vec<f32>) -> Vec<i16> {
    encoded
        .into_iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16)
        .collect()
}

// =============================================================================
// STEP 5: Write WAV file to disk
// =============================================================================

fn write_wav_file(output_path: &Path, quantized: &[i16], spec: hound::WavSpec) {
    let mut writer = WavWriter::create(output_path, spec).expect("failed to create wav writer");
    
    for &sample in quantized {
        writer.write_sample(sample).expect("failed to write sample");
    }
    
    writer.finalize().expect("failed to finalize wav file");
    println!("Wrote watermarked audio to {}", output_path.display());
}
