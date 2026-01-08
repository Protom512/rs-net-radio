# rs-net-radio

port of yayugu/net-radio-archive
## Authors

- [@protom512](https://www.github.com/protom512)

## current status
- [x] onsen
- [x] 響Radio (Partial implementation, under review)
- [x] Radiko (Partial implementation, under review)

## Requirements

This project requires the following external command-line tools to be installed on your system:

- **ffmpeg:** Used for processing and saving video/audio streams.

Please ensure these are installed and accessible in your system's PATH.

## Setup

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/your-repo-path/rs-net-radio.git
    cd rs-net-radio
    ```
2.  **Set the Archive Path:**
    This application requires the `RS_NET_ARCHIVE_PATH` environment variable to be set. This variable defines the root directory where recorded programs will be saved. Each service will have its own subdirectory created within this path.

    Example:
    ```bash
    export RS_NET_ARCHIVE_PATH="/mnt/media/radio_archives"
    ```
    Make sure this directory exists and is writable.
3.  **Build the project:**
    ```bash
    cargo build --release
    ```
4.  **Run the application:**
    ```bash
    ./target/release/rs-net-radio
    ```
