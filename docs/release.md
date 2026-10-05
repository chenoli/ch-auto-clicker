# Release

Como publicar uma versão do CH. Clicker. Tudo aqui é gratuito.

## 1. Segredos do repositório (uma vez)

Em **Settings → Secrets and variables → Actions** de `chenoli/chenoli-auto-clicker`:

| Segredo                              | Conteúdo                                                                                                                         |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `TAURI_SIGNING_PRIVATE_KEY`          | O conteúdo do arquivo `%USERPROFILE%\.tauri\ch-clicker-updater.key` (chave privada do updater, gerada em 2026-10-05, sem senha). |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Vazio (a chave não tem senha).                                                                                                   |

A chave pública correspondente já está em `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). **Guarde uma cópia da chave privada fora deste computador**: sem ela, quem já instalou o app não recebe mais atualizações (seria preciso reinstalar manualmente).

Essa chave assina só as atualizações (o app confere antes de instalar). Ela não é o certificado de assinatura de código do Windows.

## 2. Publicar uma versão

1. Atualize a versão em `src-tauri/tauri.conf.json`, `package.json` e `Cargo.toml` (`workspace.package.version`).
2. Commit na `main`, depois `git tag v0.1.0 && git push origin main --tags`.
3. O workflow `Release` roda todos os checks do CI, gera o instalador NSIS, os MSIs e o `latest.json`, e publica o release. Os apps instalados encontram a versão nova em até 24 h (ou na hora, por "Verificar atualizações" nas Configurações).

## 3. Assinatura de código do Windows (gratuita, SignPath Foundation)

Sem assinatura, o Windows mostra "Editor desconhecido" e o Controle Inteligente de Aplicativos bloqueia o app. O caminho gratuito para projetos open source:

1. Pré-requisitos já atendidos: licença MIT (aprovada pela OSI), código todo aberto, repositório público.
2. Publicar ao menos um release (o v0.1.0 sem assinatura) com a descrição do app no README.
3. Inscrever o projeto em <https://signpath.org> (formulário para projetos open source).
4. Aprovado: configurar no repositório os dados do SignPath (token de API, organização, projeto e política de assinatura) e apontar `bundle.windows.signCommand` (em `tauri.conf.json`) para um script que envia cada arquivo ao SignPath e espera o arquivo assinado.

O `signCommand` roda dentro do build do Tauri: o executável e o instalador são assinados antes de a assinatura do updater ser calculada. Assinar depois do build quebraria a verificação das atualizações.

O editor exibido no Windows passa a ser "SignPath Foundation".

## 4. Microsoft Store (opcional, gratuita)

A conta de desenvolvedor da Store é gratuita (pessoa física e empresa). Um pacote MSIX enviado à Store é assinado pela Microsoft, o que elimina os avisos do Windows para quem instala pela loja. Exige um passo extra de empacotamento MSIX, autostart pelo mecanismo do pacote e atualizações pela própria Store; há revisão da loja.
