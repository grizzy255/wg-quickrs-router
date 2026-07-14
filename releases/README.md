# Prebuilt binaries

Static Linux musl builds of `wg-quickrs` from this fork.

| File | Description |
|------|-------------|
| `wg-quickrs-latest-x86_64-unknown-linux-musl.tar.gz` | Latest build |
| `wg-quickrs-<commit>-<date>-x86_64-unknown-linux-musl.tar.gz` | Versioned snapshot |

Built from commit `0bf26ae` on 20260714 (UTC).

## Contents

- `bin/wg-quickrs` — executable
- `completions/` — shell completions (if present)

## Verify

```sh
sha256sum -c wg-quickrs-latest-x86_64-unknown-linux-musl.sha256
```

## Extract

```sh
tar -xzf wg-quickrs-latest-x86_64-unknown-linux-musl.tar.gz
sudo install -m 755 bin/wg-quickrs /usr/local/bin/
```
