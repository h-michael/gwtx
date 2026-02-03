# Installation

## From crates.io

```bash
cargo install kabu
```

## With mise

```bash
mise use -g ubi:h-michael/kabu
```

## With Nix

```bash
nix run github:h-michael/kabu

# Or install to profile
nix profile install github:h-michael/kabu
```

## From GitHub Releases

Download the latest binary from [Releases](https://github.com/h-michael/kabu/releases):

### Linux (x86_64)

```bash
curl -L https://github.com/h-michael/kabu/releases/latest/download/kabu-x86_64-unknown-linux-gnu.tar.xz | tar xJf -
sudo mv kabu /usr/local/bin/
```

### macOS (Apple Silicon)

```bash
curl -L https://github.com/h-michael/kabu/releases/latest/download/kabu-aarch64-apple-darwin.tar.xz | tar xJf -
sudo mv kabu /usr/local/bin/
```

### macOS (Intel)

```bash
curl -L https://github.com/h-michael/kabu/releases/latest/download/kabu-x86_64-apple-darwin.tar.xz | tar xJf -
sudo mv kabu /usr/local/bin/
```

### Windows (PowerShell)

```powershell
Invoke-WebRequest -Uri https://github.com/h-michael/kabu/releases/latest/download/kabu-x86_64-pc-windows-msvc.zip -OutFile kabu.zip
Expand-Archive kabu.zip -DestinationPath .
# Move kabu.exe to a directory in your PATH
```
