# GVM - Go Version Manager

A simple command-line tool to manage multiple Go versions, similar to [nvm](https://github.com/nvm-sh/nvm) for Node.js.

## Features

- 📋 **List installed versions** - See all Go versions installed on your system
- 🌐 **List available versions** - Fetch all available versions from go.dev
- 📦 **Install versions** - Install any Go version using the official method
- 🔄 **Switch versions** - Easily switch between installed versions

## Installation

### From GitHub (recommended)

```bash
cargo install --git https://github.com/Yiheng-Liu/gvm.git
```

### From crates.io

```bash
cargo install govm
```

### Build from source

```bash
git clone https://github.com/Yiheng-Liu/gvm.git
cd gvm
cargo build --release
cp ./target/release/gvm ~/go/bin/
```

## Usage

```bash
# List all installed Go versions
gvm list

# List all available versions from go.dev
gvm list-all

# Install a specific version
gvm install 1.22.11

# Switch to a specific version
gvm use 1.22.11
```

## Example Output

```
$ gvm list
Installed Go versions:
     1.22.1
     1.23.4
  -> 1.24.4 (current)
     1.25.5

$ gvm use 1.23.4
✓ Now using Go 1.23.4
go version go1.23.4 darwin/arm64
```

## How It Works

GVM uses Go's official version management approach under the hood:

1. **Install**: Runs `go install golang.org/dl/go<version>@latest` followed by `go<version> download`
2. **Use**: Creates a symlink `~/go/bin/go` → `~/go/bin/go<version>`
3. **List**: Scans `~/go/bin/` for installed Go versions

## Requirements

- Go (any version) must be installed and available in PATH
- Rust toolchain (for building from source)
