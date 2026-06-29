# Smoke Test Omarchy Package

## Preconditions
- Omarchy Linux (Arch-based) environment with `base-devel` and Rust toolchain.
- `pacman`, `makepkg`, and `sudo` available.

## Build package
```bash
./scripts/build-omarchy-pkg.sh 0.1.0
```

Expected artifact:
- `target/omarchy/build/maestro-ai-0.1.0-1-<arch>.pkg.tar.zst`

## Run automated smoke test
```bash
./scripts/smoke-test-omarchy.sh target/omarchy/build/maestro-ai-0.1.0-1-$(uname -m).pkg.tar.zst
```

## Manual verification checklist
1. Install package:
```bash
sudo pacman -U --noconfirm target/omarchy/build/maestro-ai-0.1.0-1-$(uname -m).pkg.tar.zst
```
2. Verify command surface:
```bash
maestro --help
maestro list-agents
```
3. Verify config and directories:
```bash
test -f /etc/maestro/config.yml
test -d /var/lib/maestro
test -d /var/log/maestro
```
4. Validate runtime doctor command:
```bash
maestro doctor --config /etc/maestro/config.yml
```
5. Remove package:
```bash
sudo pacman -R --noconfirm maestro-ai
```

## Expected result
- Installation succeeds in a clean Omarchy environment.
- Binary is available while package is installed.
- Default config and runtime directories are provisioned.
- Removal uninstalls the binary cleanly.
