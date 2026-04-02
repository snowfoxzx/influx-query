# AGENTS.md

This file is for AI successors working in this repository.

## Repository Conventions

- Prefer English for repository documentation and Git commit messages unless a task explicitly requires another language.
- Update `Cargo.toml` `package.version` before publishing a new release tag.

## Purpose

This repository has three coupled deliverables:

1. A Rust CLI named `influx-query` for querying InfluxDB v1 and v2.
2. A distributable skill in `skills/influx-query/` that installs and uses the CLI.
3. A GitHub Actions release workflow that publishes platform-specific binaries and `SHA256SUMS`.

For Codex compatibility, the repository ships a repo-root `.codex-plugin/plugin.json` that exposes the bundled skill for distribution.

When changing one of these areas, check whether the other two also need updates.

## Key Files

- `src/main.rs`: CLI entrypoint.
- `src/lib.rs`: argument parsing, request building, query execution, response formatting, debug output, and unit tests.
- `tests/install-script-test.sh`: shell-level tests for the skill installer naming and checksum logic.
- `skills/influx-query/SKILL.md`: distributable skill instructions.
- `skills/influx-query/scripts/install_influx_query.sh`: platform detection, release download, checksum verification, extraction, and install flow.
- `.codex-plugin/plugin.json`: minimal Codex plugin manifest that exposes `./skills/`.
- `.github/workflows/release.yml`: release pipeline for all supported targets plus `SHA256SUMS`.
- `README.md`: human-facing project and release overview.

## Invariants

- The CLI must remain a single executable with no runtime dependency on the official Influx CLI or Python scripts.
- Release asset names are part of the installer contract. Do not rename them casually.
- The install script assumes GitHub Releases contain:
  - one archive per supported OS/arch
  - one `SHA256SUMS` file
- The default release repository in the install script is `snowfoxzx/influx-query`.
- Query result behavior should not silently swallow empty results.
- `--debug` output goes to `stderr`; query results go to `stdout`.

## Common Tasks

### Change CLI behavior

1. Update logic in `src/lib.rs` and `src/main.rs`.
2. Add or adjust Rust unit tests in `src/lib.rs`.
3. Run:

```bash
cargo test
```

### Change skill installer behavior

1. Update `skills/influx-query/scripts/install_influx_query.sh`.
2. Update `skills/influx-query/SKILL.md` if invocation or guarantees changed.
3. Run:

```bash
sh tests/install-script-test.sh
```

### Change release behavior

1. Update `.github/workflows/release.yml`.
2. Verify the workflow still matches installer expectations:
   - same archive naming convention
   - `SHA256SUMS` still published
   - supported targets still align with install script platform mapping
3. If release asset naming changes, update:
   - `skills/influx-query/scripts/install_influx_query.sh`
   - `tests/install-script-test.sh`
   - `README.md`
   - `skills/influx-query/SKILL.md`

### Cut a release

1. Ensure `main` contains the desired changes.
2. Run fresh verification:

```bash
cargo test
sh tests/install-script-test.sh
```

3. Create and push a semver tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

4. Check the GitHub Actions run and confirm release assets exist before assuming the skill installer works.

## Verification Rules

Before claiming work is complete, run the relevant commands fresh.

Minimum verification for most changes:

```bash
cargo test
sh tests/install-script-test.sh
```

If you changed only documentation, say so explicitly.

If you changed release workflow logic, also inspect the workflow file and confirm it is still internally consistent with the installer script.

## Release Notes For Successors

- `v0.1.0` and `v0.1.1` had failed or cancelled release attempts during bring-up.
- `v0.1.2` was created after restoring `dtolnay/rust-toolchain@stable`.
- If a release fails, inspect the failed job logs first instead of guessing. The common failure modes so far have been:
  - unsupported GitHub runner labels
  - incorrect action input usage

## Do Not Do These Blindly

- Do not change archive names without updating installer logic and tests.
- Do not remove checksum verification from the installer.
- Do not switch default repository coordinates without confirming the canonical GitHub repo.
- Do not assume GitHub runner labels remain valid forever; verify against current GitHub docs or actual run output.
- Do not tag a new release until `main` contains the intended workflow fixes.
