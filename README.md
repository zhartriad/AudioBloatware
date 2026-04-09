# AudioBloatware

> **Elevating your daily listening experience.** > AudioBloatware is designed to solve common audio issues—like background noise, jarring volume spikes, and flat stereo imaging. By applying a chain of simple yet highly effective DSP (Digital Signal Processing) filters, it transforms standard audio into a rich, immersive soundscape. All of this runs in a safe, clean, and lightweight user-space environment, completely free of bloat.

## 🧠 How It Works (The Architecture)
Unlike traditional kernel-level audio drivers that can cause system instability, AudioBloatware operates entirely in **User Space**. 
It leverages the Linux `PipeWire` audio server to dynamically create a virtual audio cable (Null Sink). The application intercepts the raw audio stream, routes it through a lock-free ring buffer, applies mathematical transformations per-sample, and pushes the clean, enhanced audio to your physical hardware in real-time with virtually zero latency.

## 🎛️ The DSP Chain (Technical Details)
The core engine is built in Rust using `cpal` for high-performance audio I/O. The audio passes through the following pipeline:

1. **Soft Noise Gate (Envelope Follower):** Instead of a harsh cut-off that clips vocal trails, it uses an Exponential Moving Average (EMA) to track the audio's energy envelope. It smoothly attenuates the volume when the signal falls below the threshold, effectively killing background hiss and mic bleed.
2. **2-Band Parametric EQ (Crossover):** Splits the frequency spectrum into Bass and Mid/Highs. It applies a +15% gain to the low end for punchiness and a +25% presence boost to the mids/highs to ensure vocals cut through heavy bass without muddiness.
3. **Stereo-Linked Dynamic Range Compressor (DRC):** Protects your hearing from sudden loud noises (explosions, screaming) while raising the volume of quiet sounds (footsteps, whispers). It is *stereo-linked*—meaning it calculates gain reduction based on the maximum peak of both L/R channels—preventing the stereo image from drifting left or right during asymmetrical volume spikes.
4. **Mid/Side Spatializer (Stereo Widening):** Instead of using simple delay arrays (which cause phase cancellation/comb filtering on physical speakers), this filter converts the L/R signal into a Mid/Side matrix. By boosting the Side channel (+35%) and keeping the Mid channel anchored, it creates a massive, cinematic soundstage that translates perfectly to both headphones and laptop speakers.
5. **Hard Clipper (Safety Mechanism):**
   A final mathematical clamp (`-1.0` to `1.0`) ensures that no matter how hard the compressor is pushed, the digital signal will never exceed 0dBFS, protecting your physical speaker drivers from clipping damage.

## 🚀 Installation & Setup (Linux)

### Prerequisites
* **PipeWire** (and `pactl` command-line utility)
* **Rust Toolchain** (cargo)

### 1. Build from Source
Clone the repository and compile it in release mode for maximum DSP performance:
```bash
git clone https://github.com/zhartriad/AudioBloatware.git
cd AudioBloatware
cargo build --release
### 2. Manual Execution & Routing
Run the binary:
```bash
cargo run --release
```
The application will automatically create a virtual soundcard named `AudioBloatware_Virtual`. Open **Pavucontrol** (Volume Control) and ensure the following routing:
* **Playback Tab:** Route your target apps (Spotify, Browser, Games) to `AudioBloatware_Entrada`. Route the *PipeWire ALSA [AudioBloatware]* stream to your physical headphones/speakers.
* **Recording Tab:** Ensure the *PipeWire ALSA [AudioBloatware]* input is capturing the `Monitor of AudioBloatware_Entrada` (NOT your physical microphone).

## 👻 Running as a Background Daemon (Systemd)
For a seamless "set it and forget it" experience, you can run AudioBloatware as a user-level systemd service that starts automatically with your PC.

1. Move the compiled binary to your local bin path:
```bash
mkdir -p ~/.local/bin
cp target/release/AudioBloatware ~/.local/bin/AudioBloatware
```
2. Create the systemd service file:
```bash
nano ~/.config/systemd/user/AudioBloatware.service
```
3. Paste the following configuration:
```ini
[Unit]
Description=AudioBloatware DSP Filter
After=pipewire.service

[Service]
ExecStart=%h/.local/bin/audiobloatware
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
```
4. Enable and start the service:
```bash
systemctl --user daemon-reload
systemctl --user enable --now AudioBloatware.service
```

## 🛡️ Security & Performance
* **No Root Required:** Modifies audio streams strictly in user-space.
* **Command Injection Safe:** `pactl` commands are hardcoded in the Rust binary, preventing malicious input execution.
* **Lock-Free Concurrency:** Uses `ringbuf` (a lock-free ring buffer) to transfer audio chunks between the capture and playback threads, ensuring zero CPU blocking and preventing audio dropouts (xruns).

<p align="center">
  <img src="https://media4.giphy.com/media/v1.Y2lkPTc5MGI3NjExYjdoNWExNWNucjJ4dTl6MG9mYmE0dzFibzIzZ3U4ZTQ4em1meWJvZSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/SnVZO1N0Wo6u4/giphy.gif" width="600">
</p>

