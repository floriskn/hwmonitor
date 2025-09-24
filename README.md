# Rust Monitoring Library

## Status

Work in progress. Currently supports **Windows only**. Support for Linux and macOS is planned for future versions.

## Features (Windows only)

- Discover CPU packages, cores, and threads
- Read CPU core temperatures
- Read CPU package temperature

## Roadmap

- CPU load per core/thread
- Voltage and power usage
- Multi-platform support (Linux/macOS)
- GPU and other system sensors

## Contributing

This project is in early development. Contributions, suggestions, and bug reports are welcome.

## Notes

This library currently relies on Windows APIs to read CPU MSRs and requires appropriate privileges.
