# Release

How to publish a CH. Clicker version. Everything here is free.

## 1. Repository secrets (once)

Under **Settings → Secrets and variables → Actions** of `chenoli/chenoli-auto-clicker`:

| Secret                               | Value                                                                                                                          |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| `TAURI_SIGNING_PRIVATE_KEY`          | The contents of `%USERPROFILE%\.tauri\ch-clicker-updater.key` (the updater private key, generated on 2026-10-05, no password). |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Empty (the key has no password).                                                                                               |

The matching public key is already in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). **Keep a copy of the private key off this computer**: without it, existing installs stop receiving updates and users would have to reinstall by hand.

This key signs updates only (the app checks it before installing). It is not a Windows code-signing certificate.

## 2. Publishing a version

1. Bump the version in `src-tauri/tauri.conf.json`, `package.json` and `Cargo.toml` (`workspace.package.version`).
2. Commit to `main`, then `git tag v0.1.0 && git push origin main --tags`.
3. The `Release` workflow runs every CI check, builds the NSIS installer, the MSIs and `latest.json`, and publishes the release. Installed apps find the new version within 24 hours, or right away through "Check for updates" in Settings.

## 3. Windows code signing (free, SignPath Foundation)

Without a signature, Windows shows "Unknown publisher" and Smart App Control blocks the app. The free path for open-source projects:

1. Requirements already met: MIT license (OSI approved), fully open code, public repository.
2. Publish at least one release (the unsigned v0.1.0) with the app described in the README.
3. Apply at <https://signpath.org> (open-source project form).
4. Once approved, add the SignPath settings to the repository (API token, organization, project and signing policy) and point `bundle.windows.signCommand` in `tauri.conf.json` at a script that sends each file to SignPath and waits for the signed file.

`signCommand` runs inside the Tauri build, so the executable and the installer are signed before the updater signature is computed. Signing after the build would break update verification.

Windows then shows "SignPath Foundation" as the publisher.

## 4. Microsoft Store (optional, free)

A Store developer account is free for individuals and companies. An MSIX package submitted to the Store is signed by Microsoft, which removes the Windows warnings for users who install from the Store. It needs an extra MSIX packaging step, autostart through the package mechanism and updates through the Store itself, and every submission goes through Store review.
