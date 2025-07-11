# Raycast Linux

An open-source, Raycast-inspired launcher for the Linux desktop. This project is a hobbyist's effort to bring the high-speed, extensible productivity interface of Raycast to the Linux ecosystem, built with a modern tech stack.

![GIF of Raycast Linux, showing off its main features](https://raw.githubusercontent.com/ByteAtATime/raycast-linux/main/images/raycast-linux.gif)

**Disclaimer:** This is a hobby project and is **not** affiliated with, nor endorsed by, the official Raycast team.

## ✨ Features

This launcher aims to recreate most of Raycast's core features on Linux:

- **Extensible Command Palette**: The core of the application. Search for and launch applications, run commands, execute quicklinks, and more.
- **Extension Support**: Run extensions built Raycast's API. Features a built-in store to browse and install extensions from the official Raycast Store.
- **Powerful Calculator**: A smart calculator integrated directly into the search bar, powered by **SoulverCore**. It handles unit conversions, currency, and complex math expressions.
- **Clipboard History**: A searchable history of everything you've copied, with support for text, images, links, and colors.
- **Snippets**: Create and manage text snippets that can be expanded anywhere on your system. Supports dynamic placeholders for dates, clipboard content, and more.
- **AI Integration**: Connects to OpenRouter to bring the power of various AI models directly into the launcher.
- **And more** to come!

## 🧩 Extension Compatibility

While the goal is to support a wide range of Raycast extensions, there are some inherent limitations due to the differences between macOS and Linux. Common reasons an extension might not work include:

1.  **macOS-specific APIs**: Many extensions rely on native macOS features like AppleScript (`runAppleScript`), hardcoded paths (`/Applications/`), or specific system libraries that do not exist on Linux.
2.  **Native Binaries**: Extensions that bundle pre-compiled binaries for macOS will not work. Similarly, extensions that use Swift to interact with the operating system won't work either.
3.  **Assumed Permissions**: Extensions may assume they have access to macOS-specific permissioned data (like Contacts or Calendars) which have no direct equivalent.

## 🚀 Getting Started

To get the project running locally, you'll need to set up the development environment.

### Prerequisites

- **Rust**: Install via `rustup`.
- **Node.js**: Use a recent LTS version. `pnpm` is the package manager for this project (`npm i -g pnpm`).
- **Tauri Prerequisites**: Follow the official [Tauri guide](https://v2.tauri.app/start/prerequisites/) to install system dependencies
- **Swift Toolchain**: The calculator feature uses a Swift wrapper around SoulverCore.

### Installation & Running

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/ByteAtATime/raycast-linux.git
    cd raycast-linux
    ```

2.  **Install dependencies:**
    This project uses a pnpm workspace. Install all dependencies from the root directory.

    ```bash
    pnpm install
    ```

3.  **Build the Node.js sidecar binary:**
    This step compiles the JavaScript plugin host into a binary that Tauri can execute.

    ```bash
    pnpm --filter sidecar build
    ```

4.  **Run in development mode:**
    This command will launch the Tauri application with hot-reloading for the frontend.
    ```bash
    pnpm tauri dev
    ```

## 🙏 Acknowledgements

This project stands on the shoulders of giants:

- A huge thank you to the team behind the original **[Raycast](https://raycast.com)**.
- The powerful calculator is powered by **[SoulverCore](https://github.com/soulver/SoulverCore)** by Acqualia. Special thanks to Zac for getting me a Linux build -- on his vacation, no less!

## 📜 License

This project is licensed under the [MIT License](LICENSE).
