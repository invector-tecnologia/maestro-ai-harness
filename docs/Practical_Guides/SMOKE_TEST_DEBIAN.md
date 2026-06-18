# Smoke Test Debian Package

## Preconditions
- Debian/Ubuntu clean environment (VM/container recommended).
- `dpkg-deb`, `dpkg`, and Rust toolchain installed.

## Build package
```bash
./scripts/build-deb.sh 0.1.0
```

Expected artifact:
- `target/deb/maestro-ai_0.1.0_<arch>.deb`

## Run automated smoke test
```bash
./scripts/smoke-test-debian.sh target/deb/maestro-ai_0.1.0_$(dpkg --print-architecture).deb
```

## Manual verification checklist
1. Install package:
```bash
sudo dpkg -i target/deb/maestro-ai_0.1.0_$(dpkg --print-architecture).deb
```
2. Verify binary and command surface:
```bash
maestro --help
maestro list-agents
```
3. Verify generated config and directories:
```bash
test -f /etc/maestro/config.toml
test -d /var/lib/maestro
test -d /var/log/maestro
```
4. Validate runtime doctor command:
```bash
maestro doctor --config /etc/maestro/config.toml
```
5. Remove package preserving config:
```bash
sudo dpkg -r maestro-ai
test -f /etc/maestro/config.toml
```
6. Purge package and runtime data:
```bash
sudo dpkg -P maestro-ai
! test -e /etc/maestro
! test -e /var/lib/maestro
! test -e /var/log/maestro
```

## Expected result
- Installation works in a clean environment.
- Binary is available and commands execute.
- Normal removal preserves `/etc/maestro/config.toml`.
- Purge removes `/etc/maestro`, `/var/lib/maestro`, and `/var/log/maestro`.
