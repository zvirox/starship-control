<p align="center">
  <img src="assets/logo.png" width="160" alt="Starship Control Logo" />
  <h1 align="center">Starship Control</h1>
</p>

<p align="center">
  <strong>A local-first AI cockpit and floating HUD for your Linux desktop</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/starship-control/releases"><img src="https://img.shields.io/badge/Release-Latest_Packages-blue.svg" alt="Releases" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License" /></a>
  <img src="https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/Stack-Rust_·_Tauri_2_·_React_19-indigo.svg" alt="Stack" />
</p>

<p align="center">
  <a href="#what-is-starship-control">What is it?</a> ·
  <a href="#key-features">Key Features</a> ·
  <a href="#how-it-works">How It Works</a> ·
  <a href="#the-connected-app-suite">The App Suite</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#frequently-asked-questions">FAQ</a>
</p>

---

## What is Starship Control?

Most creative apps and desktop utilities live on their own islands. A screen recorder saves a video file, but leaves editing to another tool. A video editor knows nothing about your audio tracks until you manually export stems. System utilities report high disk usage, but make you run terminal commands to clean up caches.

Starship Control connects these apps into one coordinated system. It gives you a central mission control window and a lightweight floating HUD. You describe what you want done in simple words, and Starship Control plans the steps, commands the right local tools, and pauses for your approval before making any permanent changes.

You can run open source AI models completely offline on your own GPU, or connect your favorite cloud model providers.

---

## Key Features

- **Dual-Surface Interface**: A full-size 1200x800 Cockpit window for deep project planning, alongside an unobtrusive Floating HUD (summoned anytime with `Super+Shift+Space`) for quick actions.
- **Offline Local AI Models**: Download and run open GGUF models directly on your hardware using Vulkan, ROCm, or CPU fallback. Your conversations and files never leave your computer.
- **Built-in Model Downloader**: Browse Hugging Face models inside the app, download large model files in chunks with pause and resume support, and verify file integrity automatically.
- **Cloud Model Support**: Prefer hosted intelligence? Plug in an API key for Anthropic Claude, OpenAI, Google Gemini, or OpenRouter with fast streaming responses.
- **Enforced Safety Guardrails**: Read-only tasks run automatically. Any action that alters a file, changes system settings, or deletes data pauses and requests your signed confirmation on screen.
- **Direct App Orchestration**: Communicates directly with companion desktop apps over fast, private local sockets without going through internet servers or command shells.

---

## How It Works

Starship Control does not rely on third-party cloud servers to interact with your system. Everything happens right on your machine.

1. **Local App Discovery**: Companion apps publish a small manifest file in your user data directory (`$XDG_DATA_HOME/starship/plugins/`). Starship Control detects them automatically when they start.
2. **Private Unix Sockets**: Communication travels over dedicated Unix domain sockets located in `$XDG_RUNTIME_DIR/starship/bridges/`.
3. **Operating System Security**: Every connection is verified using Linux peer credentials (`SO_PEERCRED`). Only processes running under your exact Linux user account can talk to the socket.
4. **Structured Task Plans**: When you ask for something complex, the agent drafts a step-by-step plan. You can review every action before approving it.

---

## The Connected App Suite

Starship Control coordinates a complete suite of specialized Linux desktop tools:

- **Screen Studio Pro**: High-framerate desktop, window, and webcam recording on Wayland and X11.
- **Easy Edits Pro**: Non-destructive multitrack video editing, color adjustment, and export.
- **Audio Labs Pro**: Local voice recording, multitrack audio mixing, and speech synthesis.
- **Agentic Tuneup Pro**: System performance monitoring, cache cleanup, and package management with byte-for-byte rollback.
- **Net Watch Pro**: Real-time network speed tracking and per-application data usage monitoring.

You can use each app independently as a standalone program. When paired with Starship Control, they work together as a cohesive creative and maintenance workstation.

---

## Installation

Prebuilt native packages are available on the [Releases page](https://github.com/zvirox/starship-control/releases).

### Debian, Ubuntu, and Pop!_OS (`.deb`)

```bash
sudo apt install ./starship-control_*_amd64.deb
```

### Fedora, RHEL, and openSUSE (`.rpm`)

```bash
sudo dnf install ./starship-control-*.x86_64.rpm
```

### Arch Linux and Manjaro (`.pkg.tar.zst`)

```bash
sudo pacman -U ./starship-control-*-x86_64.pkg.tar.zst
```

### Building From Source

Prerequisites: Rust stable, Node.js 22+, and standard Linux development packages (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `libssl-dev`, `libsqlite3-dev`).

```bash
git clone https://github.com/zvirox/starship-control.git
cd starship-control
cd app && npm install && npm run build && cd ..
cargo build --release -p starship-control
```

---

## Frequently Asked Questions

#### Does Starship Control send my files or prompts to external servers?
No. When using a local GGUF model, every calculation and file check happens locally on your computer with zero network traffic. If you choose to configure a cloud provider like Claude or OpenAI, only the conversation prompt you send is transmitted to that provider over HTTPS.

#### Can the AI make unwanted changes to my computer?
No. Starship Control has strict safety boundaries built into its Rust core. Any action that modifies a file, alters a timeline, cleans a cache, or runs an administrative task requires your explicit confirmation on the HUD or Cockpit.

#### Do I need dedicated GPU hardware to run Starship Control?
No. While an AMD or NVIDIA GPU makes local GGUF models respond faster, Starship Control includes CPU fallback support. Alternatively, you can run the app with lightweight cloud models.

#### How do I summon the floating HUD?
Press `Super+Shift+Space` anywhere on your desktop. The HUD will appear instantly at the top of your screen, ready for voice or text input.

---

## License

Starship Control is open-source software licensed under the [Apache License, Version 2.0](LICENSE).
