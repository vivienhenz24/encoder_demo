// Import the standard library's environment module for reading command-line arguments
use std::env;

// Import modules we defined in separate files
mod decoder; // Contains all decoding logic
mod encoder; // Contains all encoding logic

// =============================================================================
// Entry point - runs encode or decode based on command
// =============================================================================

fn main() {
    // Collect all command-line arguments into a vector (first arg is program name)
    let args: Vec<String> = env::args().collect();

    // Match on the first argument to determine what mode we're in
    match args[1].as_str() {
        // If user wants to encode the message
        "encode" => {
            encoder::encode_sample("fourier");
        }

        // If user wants to decode a watermark
        "decode" => {
            // Decode the watermark from the default path
            decoder::decode_watermarked_sample(&decoder::default_watermarked_path());
        }

        // If user provided an unknown option
        _ => {
            println!("unknown option"); // Print to stderr
        }
    }
}
