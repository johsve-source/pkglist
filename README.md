## PKGLIST

A Rust-based utility that displays a colorized history of installed, upgraded, and removed packages from your Arch Linux system using pacman's log files, sorted by date.

### Features

- **Colorized Output**: Uses ANSI RGB colors for easy visual distinction
- **Parallel Processing**: Leverages Rayon for efficient log parsing
- **Caching System**: Maintains a cache file for faster subsequent runs
- **Complete History**: Shows both currently installed and previously removed packages
- **Human Readable**: Clean, formatted output with timestamps and status indicators

### Installation

##### Prerequisites

```bash
pacman -S rust git
cargo -v
```

##### Build from Source

```bash
git clone <repository-url>
cd <repository-name>
cargo build --release
```

<i>The binary will be available at target/release/pkglist</i>

##### Output

```bash
2024-01-15T14:30:45+0100 :: INS :: firefox
2024-01-16T09:15:22+0100 :: UPG :: linux
2024-01-17T16:45:33+0100 :: REM :: old-package
```
