# User-written E2E tests

This directory is reserved for **your** E2E tests against a live
Proxmox VE server. Anything you put here:

* **Survives regeneration.** The generator's orphan-removal step is
  driven by `.openapi-generator/FILES` — files outside that manifest
  (including this whole directory) are never touched by `sdk-sync`.
* **Survives the workflow wipe.** `sdk-bootstrap` only touches the
  workflow files it manages (`ci.yml`, `publish.yml`). User-added
  workflows like `.github/workflows/e2e.yml` are preserved verbatim.

The auto-generated SDK code lives outside this directory; treat
everything under `tests/e2e` as your own.

## Quickstart

1. Add fixtures, helpers, and tests under `tests/e2e/`.
2. (Optional) Add `.github/workflows/e2e.yml` to run them in CI against
   a Proxmox VE test instance — the sync pipeline preserves
   any workflow file whose name isn't in the managed allowlist.
3. The generated `ci.yml` runs the SDK's own build + unit tests; wire
   your E2E suite into a separate workflow so it can use its own
   secrets / runners / schedule.

## Conventions per language

| Language | User-test path | Runner |
|---|---|---|
| TypeScript | `tests/e2e/*.test.ts` | `vitest run tests/e2e` |
| Python | `tests/e2e/test_*.py` | `pytest tests/e2e` |
| Go | `tests/e2e/*_test.go` (package `e2e_test`) | `go test ./tests/e2e/...` |
| Rust | `tests/e2e/*.rs` | `cargo test --test '*'` |
| PHP | `tests/E2E/*Test.php` | `vendor/bin/phpunit tests/E2E` |
| Kotlin | `src/test/kotlin/e2e/*Test.kt` | `./gradlew test --tests 'e2e.*'` |

> The Kotlin generator's post-process step deletes
> `src/test/kotlin/**/apis/*.kt` and `src/test/kotlin/**/models/*.kt`
> (openapi-generator's stub tests). Keep your tests **outside** those
> two subpaths — the `e2e/` directory recommended above is the
> intended location and is never touched by the pipeline.

## Authenticating against a real server

Use the API-token format documented in the SDK's main README:

* Perl family (PVE, PMG): `<PREFIX>APIToken=USER@REALM!TOKENID=UUID`
* Rust family (PBS, PDM): `<PREFIX>APIToken=USER@REALM!TOKENID:UUID`

Set the secret via env var so tests stay portable:

```sh
PVE_HOST=https://pve.example.com:8006
PVE_TOKEN='PVEAPIToken=root@pam!auto=<your-uuid>'
PVE_INSECURE_TLS=1   # only when targeting a self-signed dev server
```
