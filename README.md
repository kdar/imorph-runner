# iMorph Runner

iMorph-runner is a utility for managing and launching iMorph. It checks for updates, downloads them if available, and then runs iMorph with the latest version.

## Features

- Automatically checks for iMorph updates.
- Downloads the latest iMorph release if available.
- Launches iMorph after update check/download.
- Configurable via aTOML file.
- Supports multiple products (Retail, Classic, and Classic Era).
- Support multiple regions (Global and China).

## Prerequisites

1. You must download and install [MEGAcmd](https://mega.io/cmd). Either ensure mega-get.bat is in your PATH, or configure mega_get to be the full path in config.toml.
2. Login to mega using `mega-login --% "<username>" "<password>"`.
3. Add the iMorph folder using `mega-import --% "https://mega.nz/folder/XQdwFJTR#X8VNWdap7eKtIvmPbpW6sA".`

## Installation

Download the imorph-runner-\*.zip file from https://github.com/kdar/imorph-runner/releases/latest and unzip it somewhere.

## Usage

Run the program by double clicking it or running imorph-runner.exe in the terminal.

## Configuration

Configure behavior by editing `config.toml`:

```toml
region = "global"                       # "global", "china". Defaults to "global".
product = "wow"                         # "wow", "wow_classic", "wow_classic_era". Defaults to "wow".
feature = "net"                         # "none", "net", "menu". Defaults to "net".
output_directory = "download"           # Defaults to "download".
mega_path = ""                          # Path where MEGAcmd is installed. Default is to assume the commands are in $PATH.
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
