# iMorph Runner

iMorph-runner is a utility for managing and launching iMorph. It checks for updates, downloads them if available, and then runs iMorph with the latest version.

## Features

- Automatically checks for iMorph updates
- Downloads the latest iMorph release if available
- Launches iMorph after update check/download
- Configurable via TOML vile
- Supports multiple flavors and regions (e.g., Retail, Classic)

## Prerequisites

You must download and install [MEGAcmd](https://mega.io/cmd). Either ensure mega-get.bat is in your PATH, or configure mega_get to be the full path in config.toml.

## Usage

Just run the program by double clicking it or running imorph-runner.exe in the terminal.

## Configuration

Configure behavior by editing `config.toml`:

```toml
region = "Global"                       # "Global", "China". Defaults to "Global".
flavor = "Retail"                       # "Retail", "Classic", "Classic Era". Defaults to "Retail".
name = "iMorph Net"                     # "iMorph", "iMorph Net", "iMorph Menu". Defaults to "iMorph Net".
output_directory = "download"           # Defaults to "download".
mega_get = "mega-get.bat"               # Path to the mega-get.bat from MEGAcmd. Defaults to "mega-get.bat".
api = "https://www.imorph.dev/api/apps" # URI to API. Defaults to "https://www.imorph.dev/api/apps".
```

## Building

```sh
# Clone repository
git clone https://github.com/kdar/imorph-runner.git
cd imorph-runner

# Build with Cargo
cargo build --release
```

## Contributing

Software contributions welcome! Feel free to open issues or submit pull requests.

## License

This project is licensed under the MIT License – see the [LICENSE](LICENSE).
