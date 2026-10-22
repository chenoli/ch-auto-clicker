# Contributing to CH Auto Clicker

Thanks for your interest in improving CH Auto Clicker. Bug reports, translations, documentation fixes and code are all welcome.

## Before you start

- Search the [existing issues](https://github.com/chenoli/ch-auto-clicker/issues) first.
- For anything bigger than a small fix, open an issue to discuss the idea before writing code.
- Security problems go through [`SECURITY.md`](SECURITY.md), never a public issue.
- By contributing you agree that your work is released under the [MIT license](LICENSE) and that you follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Scope

CH Auto Clicker is a general-purpose automation tool. Contributions that help hide automated input from games, anti-cheat systems or other detection mechanisms will not be accepted. The emergency stop must always work and can never be made optional.

## Setting up

Follow the requirements and commands in the [README](README.md#development). In short:

```sh
pnpm install
pnpm tauri dev
```

Windows 11 Smart App Control blocks locally built executables, so it must be off for development.

## Making a change

1. Fork the repository and create a branch from `main`.
2. Keep the change focused. One pull request per topic.
3. Add or update tests next to the code you change. Coverage must stay at or above 80% (frontend and Rust crates).
4. Run the full check before opening the pull request:

   ```sh
   pnpm lint && pnpm format:check && pnpm typecheck && pnpm test && pnpm test:visual
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

5. If you touch the UI, include before/after screenshots in the Grafite (dark) and Papel (light) themes.

Tests that inject real mouse input are marked `#[ignore]`. Only run them with games and sensitive apps closed.

## Code style

- Prettier and rustfmt decide formatting; ESLint and Clippy run with zero warnings allowed.
- File names are kebab-case (snake_case in Rust).
- Code carries no comments. Let names explain intent. The only exception is `// SAFETY:` on every `unsafe` block.
- Colors and type sizes come from the design tokens (`design/tokens.json`, regenerated with `pnpm tokens`); never use arbitrary Tailwind values.
- Every user-facing string goes through i18n and must exist in all locales (`ui/src/i18n/locales/`). Native strings drawn by Rust live in `src-tauri/src/native_text.rs`.

## Translations

The UI ships in Brazilian Portuguese (`pt-BR`), English (`en-US`) and Latin American Spanish (`es-419`). To improve a translation, edit the matching `common.json`. To add a language, open an issue first: it also needs native strings, an installer language and a new `Locale` variant.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/) in English, for example `fix(hud): keep the saved position on a disconnected monitor`. Explain the why in the body when it is not obvious.

## Pull requests

Fill in the pull request template. CI must be green before review. A maintainer may ask for changes or squash the commits when merging.
