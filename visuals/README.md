# Audio Watermark Visualization

This directory contains a Jupyter notebook for visualizing the audio watermark encoding process.

## Setup

### 1. Create a Python virtual environment

```bash
cd visuals
python3 -m venv venv
source venv/bin/activate  # On macOS/Linux
# OR
venv\Scripts\activate  # On Windows
```

### 2. Install dependencies

```bash
pip install -r requirements.txt
```

### 3. Launch Jupyter

```bash
jupyter notebook audio_visualization.ipynb
```

Or use JupyterLab:

```bash
jupyter lab
```

## What the Notebook Shows

The notebook provides several visualizations:

1. **Time-Domain Waveforms**: Compare original vs watermarked audio
   - Shows the audio signal over time
   - Highlights the (nearly invisible) watermark differences

2. **FFT Spectrum Analysis**: See the frequency domain representation
   - Full spectrum view highlighting the watermark region (bins 10-128)
   - Zoomed view showing individual frequency bins
   - Frequency resolution: 31.25 Hz per bin

3. **Watermark Bit Visualization**: Decode and display the embedded bits
   - Shows the magnitude ratio (watermarked/original)
   - Verifies the pilot pattern: `[0, 1, 0, 1, 0, 1, 0, 1]`
   - Expected ratios: 0.85x for bit 0, 1.15x for bit 1

4. **Spectrograms**: Time-frequency representation
   - Shows how frequency content evolves over time
   - Highlights the watermark frequency range (312.5 Hz - 4000 Hz)

5. **Lab Equipment Guide**: Measurement reference table
   - Lists exact frequencies for each bin
   - Shows expected patterns for spectrum analyzer measurements
   - Provides guidance for manual decoding with lab equipment

## Using This for Your Lab

The visualizations will help you:

- **Understand where the watermark lives** in the frequency domain
- **Identify the exact frequencies** to measure with a spectrum analyzer
- **Verify the pilot pattern** to calibrate your measurements
- **See the magnitude differences** between original and watermarked signals

### Key Frequencies for Lab Measurements

| Bin | Frequency | Pilot Bit |
|-----|-----------|-----------|
| 10  | 312.5 Hz  | 0 (LOW)   |
| 11  | 343.75 Hz | 1 (HIGH)  |
| 12  | 375.0 Hz  | 0 (LOW)   |
| 13  | 406.25 Hz | 1 (HIGH)  |
| ... | ...       | ...       |

## Notes

- The notebook automatically loads audio from `../input_data/` and `../output_data/`
- You can modify the code cells to analyze different portions of the audio
- Try changing `start_sample` in cell 7 to analyze different frames
- Adjust `bins_to_analyze` in cell 9 to see more/fewer watermark bins

## Troubleshooting

**If matplotlib plots don't show:**
```bash
pip install --upgrade matplotlib
```

**If seaborn style errors occur:**
- The notebook uses `seaborn-v0_8-darkgrid` style
- If you have an older version, change it to `'seaborn'` or `'default'` in cell 1

**If audio files not found:**
- Make sure you've run the Rust encoder first to generate the watermarked audio
- Check that paths in cell 3 match your file structure

