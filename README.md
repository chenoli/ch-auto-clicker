# CH. Clicker

Auto clicker nativo para Windows 10/11, da Chenoli Labs. Núcleo em Rust (injeção via `SendInput`, scheduler de alta precisão) e interface em React sobre Tauri 2.

- Arquitetura: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Backlog em fases: [`docs/TASKS.md`](docs/TASKS.md)
- Regras de trabalho: [`CLAUDE.md`](CLAUDE.md)

## Pré-requisitos

- Windows 10 1803+ ou Windows 11
- [Rust](https://rustup.rs) stable (MSVC) e Visual Studio Build Tools com a carga "Desenvolvimento para desktop com C++"
- Node.js 24 e pnpm 12
- WebView2 (já vem no Windows 11)

O **Controle Inteligente de Aplicativos** do Windows 11 bloqueia os executáveis gerados pelo build. Para desenvolver, ele precisa estar desligado.

## Comandos

| Comando                                                                  | O que faz                                                                     |
| ------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| `pnpm install`                                                           | Instala as dependências do frontend                                           |
| `pnpm tauri dev`                                                         | Abre o app em modo de desenvolvimento                                         |
| `pnpm dev`                                                               | Só o frontend, no navegador (galeria em `http://localhost:1420/gallery.html`) |
| `pnpm test`                                                              | Testes do frontend com cobertura (mínimo 80%)                                 |
| `pnpm test:visual`                                                       | Paridade visual com Playwright                                                |
| `pnpm lint` / `pnpm lint:fix`                                            | ESLint estrito (zero warnings)                                                |
| `pnpm format`                                                            | Prettier                                                                      |
| `pnpm typecheck`                                                         | TypeScript                                                                    |
| `pnpm tokens`                                                            | Regenera temas a partir de `design/tokens.json`                               |
| `cargo test --workspace`                                                 | Testes Rust (também regenera `ui/src/ipc/bindings.ts`)                        |
| `cargo clippy --workspace --all-targets -- -D warnings`                  | Lint Rust                                                                     |
| `cargo llvm-cov --workspace --exclude ch-clicker --exclude jitter-bench` | Cobertura Rust (mínimo 80%)                                                   |
| `cargo test -p ch-win --test injection -- --ignored`                     | Testes que injetam cliques reais (só com jogos e apps sensíveis fechados)     |
| `cargo run -p jitter-bench --release -- --seconds 10`                    | Benchmark de jitter do scheduler (relatório em `docs/bench/`)                 |

## Estrutura

```
crates/ch-core     domínio puro (sem Windows): modelos, padrões, scheduler
crates/ch-win      camada Windows: input, timers, DPI, hotkeys, captura
crates/ch-engine   runtime do motor
crates/ch-storage  persistência em %APPDATA%
tools/jitter-bench benchmark de timing do scheduler
src-tauri/         app Tauri: comandos, eventos, janelas
ui/                interface React (uma página por janela)
design/            tokens (fonte da verdade) e ícone
tools/             gerador de tokens e regras de lint locais
```

## Aviso

Sistemas anti-cheat detectam entrada automatizada. O CH. Clicker não oculta sua atividade. Não use onde os termos de serviço proíbem automação.
