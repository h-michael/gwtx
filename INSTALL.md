# Installation

## From crates.io

```bash
cargo install gwtx
```

## With mise

```bash
mise use -g ubi:h-michael/gwtx
```

## With Nix

```bash
nix run github:h-michael/gwtx

# Or install to profile
nix profile install github:h-michael/gwtx
```

## From GitHub Releases

Download the latest binary from [Releases](https://github.com/h-michael/gwtx/releases):

### Linux (x86_64)

```bash
curl -L https://github.com/h-michael/gwtx/releases/latest/download/gwtx-x86_64-unknown-linux-gnu.tar.xz | tar xJf -
sudo mv gwtx /usr/local/bin/
```

### macOS (Apple Silicon)

```bash
curl -L https://github.com/h-michael/gwtx/releases/latest/download/gwtx-aarch64-apple-darwin.tar.xz | tar xJf -
sudo mv gwtx /usr/local/bin/
```

### macOS (Intel)

```bash
curl -L https://github.com/h-michael/gwtx/releases/latest/download/gwtx-x86_64-apple-darwin.tar.xz | tar xJf -
sudo mv gwtx /usr/local/bin/
```

### Windows (PowerShell)

```powershell
Invoke-WebRequest -Uri https://github.com/h-michael/gwtx/releases/latest/download/gwtx-x86_64-pc-windows-msvc.zip -OutFile gwtx.zip
Expand-Archive gwtx.zip -DestinationPath .
# Move gwtx.exe to a directory in your PATH
```
