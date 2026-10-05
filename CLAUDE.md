<!-- dev-flow:start -->
<!-- Gerado por /dev-flow:onboarding em 2026-10-04 (opções recomendadas, decididas pelo assistente com aval do usuário). Refaça o onboarding para mudar; edições manuais aqui são preservadas até lá. -->

# CH. Clicker: regras de trabalho (dev-flow)

Regras permanentes. Não são sugestões: siga exatamente.

## 1. Parar a cada fase

Nunca inicie a próxima fase por conta própria, mesmo que a anterior tenha fechado limpa. Ao terminar uma fase:
implementar → verificar → revisão em 10 camadas → corrigir → atualizar `refs/` → commit → **parar e esperar autorização explícita**.
Um "sim" sobre o roadmap geral não é cheque em branco. Se a sessão for interrompida no meio de uma fase, retome a mesma fase.

## 2. Commits

- Padrão: Conventional Commits (`feat:`, `fix:`, `chore:`, `test:`, `docs:`, `refactor:`), com escopo da fase quando útil (`feat(engine): ...`).
- Granularidade: um commit por fase, com corpo em tópicos explicando o quê e o porquê, mais as limitações aceitas.
- Idioma das mensagens: português.
- Branches: direto na `main` (projeto solo, sem remote por enquanto).
- Nunca push, PR, `--amend` ou `--no-verify` sem pedido explícito.

## 3. Revisão em 10 camadas + lint --fix

Antes de declarar qualquer fase pronta, revisar e checar de fato (`npm view`, `pnpm ls`, `pnpm audit`, `cargo tree`, `grep`/`find`):
arquitetura · segurança · dependências · banco (aqui: persistência em disco e schema JSON) · qualidade/TypeScript/Rust · convenções · build/tooling · infra (aqui: instalador, assinatura, CI) · testes · documentação/DX.
Rodar `pnpm lint:fix`, `pnpm format` e `cargo fmt --all` como parte da revisão. A fase fecha com zero warnings (`eslint --max-warnings 0`, `cargo clippy --workspace --all-targets -- -D warnings`).

## 4. Projeto

- Formato do repo: app único. Workspace Cargo (`crates/*` + `src-tauri`) e frontend em `ui/` com pnpm.
- Stack: Tauri 2 + Rust (`windows-rs`) no núcleo; React 19 + TypeScript 6 + Vite 8 + Tailwind v4 na UI. Arquitetura completa em `docs/ARCHITECTURE.md`; backlog em `docs/TASKS.md`.
- Design system: handoff de alta fidelidade em `refs/` (`refs/README.md`, `refs/design/CH Clicker.dc.html`). Tokens versionados em `design/tokens.json` (fonte da verdade, gera `themes.css`, `tokens.ts` e `tokens.rs` via `pnpm tokens`). Ler a spec da tela no README do handoff antes de construí-la.
- Idiomas: conversa, UI, commits e documentos (`docs/`, `refs/`) em pt-BR; comentários de código, mensagens internas de erro e nomes no código em inglês; i18n da UI: pt-BR e en-US (i18next, chaves tipadas).

## 5. Convenções de código

- Nomes de arquivo: kebab-case, forçado por lint (`eslint-plugin-check-file`). Rust segue snake_case.
- Formatação: Prettier (aspas duplas, ponto e vírgula, 120 colunas); `rustfmt` no Rust.
- Comentários: só quando explicam um porquê não óbvio, em inglês. Blocos `unsafe` sempre com `// SAFETY:`.
- Lint: estrito. `react-hooks` como error, sem `any`, sem `console` fora de `tools/` e `ui/tests/`, `import type` separado, ordem de imports forçada (perfectionist). Clippy com `-D warnings`.
- Ordem de imports: side-effects → react → externos → contextos do app (`components`, `dev`, `features`, `i18n`, `ipc`, `store`, `styles`) → relativos → `import type` por último.
- Tokens de design: nunca valores arbitrários do Tailwind (`text-[12px]`). Cores vêm de `design/tokens.json`; escala tipográfica e tracking ficam no `@theme` de `ui/src/styles/base.css`. Misturas pontuais do README usam `color-mix()` local, sem criar nome de token novo.

## 6. Testes e verificação

- Teste co-localizado para todo código novo, na mesma fase. Cobertura mínima de 80% em linhas, branches, funções e statements (quebra o pipeline): Vitest no frontend; `cargo llvm-cov` nos crates com lógica a partir da Fase 1.
- Telas: verificar na janela real do Tauri e na galeria (`ui/gallery.html`), nos temas Grafite (escuro) e Papel (claro), com screenshot aberto e conferido; paridade com o protótipo via Playwright.
- **Input real (cliques, movimentos, hotkeys):** testes que injetam input no sistema ficam com `#[ignore]` e só rodam com autorização explícita do usuário na hora, com jogos e apps sensíveis fechados. Nunca trazer janelas para frente nem injetar input enquanto o usuário pode estar usando outro app.
- Benchmarks de timing (`tools/jitter-bench`) registram resultados em `docs/bench/`.

## 7. Segurança

- Parada de emergência nunca pode ser desativada nem bloqueada (ARCHITECTURE §10.1).
- O app nunca clica na própria janela e não oculta sua atividade (eventos marcados como injetados).
- Entradas de arquivo (import `.chprofile`) validadas pelo conteúdo e pelo schema, nunca pelo que o arquivo declara.
- Assinatura de código e updater dependem de segredos reais e ficam para a Fase 13.

## 8. Gestão de tarefas

- Tracker: só o backlog local (`docs/TASKS.md`, versionado) e o andamento em `refs/architecture/plano-tarefas.md`.
- Escrita no tracker: não se aplica.
- `/dev-flow:feature <descrição>` inicia uma feature fora do backlog; o plano fica em `refs/features/`.

## 9. Referências

- `refs/` (fora do git): handoff de design, `architecture/plano-tarefas.md` (andamento das fases), `features/`, `third-party/`. Índice em `refs/INDEX.md`.
- Atualizar o andamento a cada fase, mesmo sem versionar. O histórico oficial é o `git log`.

<!-- dev-flow:end -->
