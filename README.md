# UNDER CONSTRUCTION. WATCH OUT FOR FALLING GEESE. 

# IrateGoose - Virtual Surround Sound for PipeWire

IrateGoose is a graphical application that configures PipeWire to create a virtual sound card providing spatial audio directionality in headphones. It transforms standard 7.1 surround sound into binaural audio using Head-Related Transfer Function (HRTF) technology, giving you immersive 3D audio perception through regular stereo headphones.

<img width="1028" height="797" alt="irate_goose_mainwindow" src="https://github.com/user-attachments/assets/f2029404-be7a-469b-baea-fa3a9a1a2519" style="width:50%; height:auto;"/>

## How It Works

IrateGoose creates a virtual PipeWire sink that processes 7.1 channel audio through a **convolver** using **HRTF impulse response (IR) files**. Here's what these terms mean:

### HRTF (Head-Related Transfer Function)
HRTF is a mathematical model that describes how sound reaches your ears from different directions in space. It accounts for the shape of your head, ears, and torso, which affect how you perceive sound direction. By applying HRTF processing to surround sound, you can experience convincing 3D audio through headphones.

### Convolver
A convolver is a digital signal processing component that applies an impulse response to an audio signal. In this context, it takes the 7.1 channel audio and "convolves" it with HRTF data to create binaural output that mimics how sound would arrive at your ears from different directions.

### Impulse Response (IR) File
An impulse response file (typically a WAV file) contains the acoustic "fingerprint" of how a sound system (or in this case, a human hearing system) responds to an impulse. IR files contain measurements of how sound from each direction reaches both ears. IrateGoose uses multi-channel WAV files where each channel corresponds to a different speaker position.

### Compatibility with HeSuVi
IrateGoose uses the same mathematical processing and the same IR file format as the popular **HeSuVi** (Headphone Surround Virtualization) software. If you're familiar with HeSuVi, you'll find that IrateGoose produces similar audio quality and uses the same IR files. This means you can use your existing HeSuVi IR collection with IrateGoose.

## Installation

### Prerequisites
- **Linux** with **PipeWire** audio system (most modern Linux distributions use PipeWire by default)
- **5.1/7.1 channel audio source** (games, media players, etc.)
- **Stereo headphones** (any quality will work, but better headphones provide better results)

### Step 1: Download IrateGoose
Download the latest binary from the [Releases page](https://github.com/Barafu/IrateGoose/releases)

### Step 2: Obtain IR Files
IrateGoose requires impulse response files in WAV format compatible with HeSuVi. You have several options:

1. **Download from collection**: Get IR files from curated database:
   [https://airtable.com/appayGNkn3nSuXkaz/shruimhjdSakUPg2m/tbloLjoZKWJDnLtTc](https://airtable.com/appayGNkn3nSuXkaz/shruimhjdSakUPg2m/tbloLjoZKWJDnLtTc)

2. **Extract from HeSuVi**: If you already have HeSuVi installed, you can use the IR files from its `HeSuVi/Common/` directory. These are typically located at:
   - Windows: `C:\Program Files\EqualizerAPO\config\HeSuVi\Common\`
   - You can copy the WAV files from there to use with IrateGoose.

3. **Use your own**: Any multi-channel WAV file in HeSuVi format (14 channels for 7.1 processing) will work.

### Step 3: Installation Options

IrateGoose offers two installation approaches:

#### Option A: Try Out the Application
If you want to try IrateGoose without installing it permanently:
1. Download the binary to any location
2. Start the application
3. On the **Options tab**, select the folder containing your WAV files

#### Option B: Install for Regular Use
For permanent installation:
1. Move the binary to a directory on your PATH (recommended: `~/.local/bin/`)
2. Run the installation command to create a system menu entry:
   ```bash
   irate_goose --install
   ```
   This creates a `.desktop` file in the appropriate location for your desktop environment.

3. To uninstall (removes only the menu entry):
   ```bash
   irate_goose --uninstall
   ```
   Note: The binary and WAV files need to be removed manually if desired.

### Application icon:

<img width="512" height="512" alt="irate_goose_logo" src="https://github.com/user-attachments/assets/d3019976-6a3d-46cd-a726-30da7dc8a80a" style="width:30%; height:auto;"/>


### Command Line Options
IrateGoose supports several command-line options:

- **Set WAV folder temporarily**: Specify the WAV folder path as an argument:
  ```bash
  irate_goose /path/to/your/wav/files
  ```
  This sets the WAV folder for this run only.

- **Install/uninstall menu entry**:
  ```bash
  irate_goose --install    # Create system menu entry
  irate_goose --uninstall  # Remove system menu entry
  ```

- **Help**: Display help information:
  ```bash
  irate_goose --help
  ```

## Configuration

### Select an IR File
Launch IrateGoose. The application will display a list of detected WAV files with their descriptions:
1. Browse through the available IR files
2. Use the search box to find specific files
3. Filter by sample rate (48000 Hz, 44100 Hz, 96000 Hz, or all)
4. Select the IR file you want to use by clicking on it

The application can recognize some well-known IR files (by file name only) and show additional data:
- **HRTF name** (e.g., SADIE, MIT, etc.)
- **Description** of the measurement subject or method
- **Source** and **credits** for the data

### Configure Options (Optional)
Before applying configuration, you can customize settings on the **Options tab**:
- **Virtual Device Name**: Choose a custom name for your virtual sound card
- **WAV Folder**: Set the directory containing your WAV files

### Step 4: Apply Configuration
Click the **"Create Device"** button to apply your selection. IrateGoose will:
1. Create a PipeWire configuration file at `~/.config/pipewire/pipewire.conf.d/sink-virtual-surround-7.1-hesuvi.conf`
2. Restart PipeWire services to apply the changes
3. Create a virtual sound card with your chosen name (default: "Virtual Surround Sink")

**Important**: You can now close IrateGoose - it doesn't need to keep running! The configuration persists until you change or delete it.

### Step 4: Select the Virtual Sound Card
1. Open your desktop environment's sound settings (KDE System Settings, pavucontrol,  etc.)
2. Look for "Virtual Surround Sink" in the output devices list
3. Select it as your playback device

**Note for KDE Plasma users**: Some desktop environments, like KDE Plasma, may not show virtual sound cards by default. You may need to enable "Show virtual devices" in the sound settings.

### Step 5: Configure Your Audio Sources
For spatial audio to work correctly, your applications must output **7.1 channel audio**, not stereo or headphone audio:

#### Games:
- Look for audio settings labeled "7.1 Surround", "Studio Speakers", "Reference Speakers", or "Home Theater"
- **Avoid** settings labeled "Headphones", "Stereo", or "2.0"
- Common settings:
  - **Windows Sonic** or **Dolby Atmos for Headphones**: Disable these if using IrateGoose
  - **Speaker Configuration**: Set to "7.1 Surround" or "7.1 Speakers"
  - **Audio Output**: Set to "Speakers" not "Headphones"

In fact, failing to configure the game, and using two surround emulations at the same time, unlocks a secret Knight Mode™: an authentic feeling of a steel bucket on your head.  

#### Media Players:
- Configure to output multi-channel audio (not downmixed to stereo). Avoid upmixing stereo to 7.1 too as it may produce excessive echo. Stereo content should remain stereo, and you will hear it as if you are listening to 2.1 speakers. 

#### System-Wide:
- Ensure your system audio settings are configured for 7.1 output when using the virtual sink

## Finding the Right IR File

**The perception of spatial audio cues is as personal as a sense of smell.** Different HRTF measurements work better for different people due to variations in head shape, ear anatomy, and personal preference.

### Recommendations:
1. **Start with popular measurements**: Atmos or dc+ are good starting points
2. **Try different types**: Some are measured on human subjects, others on dummy heads (anthropomorphic manikins), some are crafted theoretically.
3. **Test with familiar content**: Use games or movies you know well to judge spatial accuracy

### What to listen for:
- **Directional accuracy**: Can you pinpoint where sounds are coming from?
- **Distance perception**: Do far sounds sound distant and near sounds close?
- **Comfort**: Does the audio feel natural or strained?
- **Frequency balance**: Does it sound tinny, boomy, or balanced?

Expect to spend some time trying different IR files until you find one that suits your hearing. What works perfectly for one person may sound unnatural to another.

## Troubleshooting

### Virtual Sound Card Not Appearing
- **KDE Plasma**: Enable "Show virtual devices" in sound settings
- **Restart audio**: Run `systemctl --user restart wireplumber pipewire pipewire-pulse`
- **Check configuration**: Ensure IrateGoose successfully wrote the config (look for `~/.config/pipewire/pipewire.conf.d/sink-virtual-surround-7.1-hesuvi.conf`)

### No Sound or Distorted Audio
- Verify you've selected your virtual sound card as output device (default name: "Virtual Surround Sink", but you can customize it on the Options tab)
- Check that your application is outputting 7.1 audio, not stereo
- Try a different IR file (some may be incompatible or damaged)
- Ensure your headphones are properly connected

### Application Errors
- **"Can not find wave files"**: IrateGoose does not automatically search its own directory for WAV files. You need to:
  1. Set the WAV folder on the **Options tab** in the application, OR
  2. Specify the folder path as a command-line argument when launching: `irate_goose /path/to/wav/files`
- **Permission errors**: Run with appropriate permissions for writing to `~/.config`
- **PipeWire not running**: Ensure PipeWire is installed and running on your system

## Removing Configuration

To remove the virtual surround sink and return to normal audio:
1. Launch IrateGoose
2. Click the **"Remove device"** button
3. The virtual sound card will be removed after PipeWire services restart
4. Select your original audio device in system settings

## Technical Details

### PipeWire Configuration
IrateGoose creates a PipeWire filter chain that:
- Accepts 8-channel input (7.1 surround: FL, FR, FC, LFE, RL, RR, SL, SR)
- Applies convolution with the selected HRTF IR file
- Mixes down to 2-channel binaural output
- Creates both input (`effect_input.virtual-surround-7.1-hesuvi`) and output (`effect_output.virtual-surround-7.1-hesuvi`) nodes

## For Packaging

If you're packaging IrateGoose for distribution, note these dependencies:

### Runtime Dependencies
- **zstd**: Required for decompressing embedded data
- **Rust-winit requirements**: Standard windowing system dependencies (X11/Wayland libraries)
- **xdg-portals**: Used for opening directory selection dialogs
- **xdg-utils**: Required for creating system menu entries via `--install`/`--uninstall` commands

### Build Dependencies
- **Rust toolchain** (latest stable)
- **Cargo** build system
- **zstd development libraries**

### Packaging Notes
- The application includes compressed data that requires zstd for decompression
- Menu integration uses standard XDG desktop entry specification
- Directory selection relies on xdg-portals for sandbox compatibility

## Building from Source

If you prefer to build from source instead of using the pre-built binary:

```bash
# Clone the repository
git clone https://github.com/???/IrateGoose.git
cd IrateGoose

# compress the data
cd data
bash compress_to_zstd.sh
cd ..

# Build with Cargo
cargo build --release

# The binary will be at target/release/IrateGoose
```

## License

IrateGoose is licensed under the MIT License. See [LICENSE](LICENSE) for details.
The application is developed with the use of DeepSeek LLM. 

## Acknowledgments

- **PipeWire** developers for the excellent audio system
- **HRTF researchers** who have made their measurements publicly available
- All contributors and testers who help improve IrateGoose
- **HeSuVi** for pioneering HRTF-based virtual surround on Windows

## Support and Feedback

Found a bug? Have a feature request? Please open an issue on the GitHub repository.

So, why IrateGoose?  **I**mpulse **R**esponse, IR. I mean, have you ever seen a goose? They have teeth! On the tongue! 

