# Smoke Test macOS Package

## Preconditions
- macOS environment with Rust toolchain installed.
- `pkgbuild` available (typically included with macOS or Xcode Command Line Tools).

## Build package
```bash
./scripts/build-macos-pkg.sh 0.1.0
```

Expected artifact:
- `target/macos/build/maestro-ai-0.1.0-macos-<arch>.pkg`

## Manual verification checklist
1. Install package:
```bash
sudo installer -pkg target/macos/build/maestro-ai-0.1.0-macos-$(uname -m).pkg -target /
```
2. Verify binary and command surface:
```bash
maestro --help
maestro list-agents
```
3. Remove package:
macOS doesn't have an automated uninstaller for `.pkg` installations. To clean up:
```bash
sudo rm /usr/local/bin/maestro
sudo pkgutil --forget com.invector.maestro-ai
```

## Expected result
- Installation succeeds cleanly on macOS.
- Binary is successfully exposed globally in the path.
- Package uninstallation cleanly forgets the package ID.