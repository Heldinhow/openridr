# SPEC — QA top 4 (achados 1–4 de `QA-BUGHUNT-2026-11-02.md`)

Escopo: só os achados 1 a 4. Os demais (5–23) ficam fora.

## 1. Análise (csharp-ls) sem caminho escondido

- Com a workspace confiada, `csharp-ls` verificado, SDK do projeto resolvido e restore atual, a UI chama `lsp.start` sozinha, **uma vez por workspace**. Parar ou falhar não reinicia sozinho.
- A barra de status ganha um item sempre visível (só em Trusted mode): `status-analysis` com o estado e, quando útil, um botão `status-analysis-action`:
  - ferramenta não instalada → **Install analysis**: abre o painel Workspace tools com o plano de instalação aberto. Nunca instala: quem confirma é o botão "Confirm and install" do plano.
  - parada/falha/ainda não iniciada → **Start analysis** (desabilitado, com o motivo no `title`, enquanto restore/SDK não estiverem prontos).
- O bloqueio de início é uma função única (`lspStartBlocker`), usada pelo painel e pela barra.

## 2. Diagnósticos no editor

- O editor desenha sublinhado ondulado (`.cm-diagnostic-error|warning|info`) e marcador na margem (`.cm-diagnostic-marker`) para cada item do arquivo ativo vindo de (a) diagnósticos do LSP em estado `current` e (b) problemas do Build que não estejam stale/sealed/obsoletos. Itens duplicados (mesmo caminho, posição e código) contam uma vez.
- A barra de status mostra a contagem (`status-diagnostics`: "N errors · M warnings"); clique abre a aba Problems.
- Sem dependência nova: usa `Decoration`/`gutter` do `@codemirror/view`.

## 3. Flag "modified"

- `Tab` guarda `savedContent` (texto na última leitura/gravação). Dirty = `content !== savedContent` ou conflito de disco. `savedRevision` deixa de existir.
- Undo até o texto salvo limpa a aba, o contador do status e os diálogos de Build/Run/fechar.
- Edições aplicadas a um arquivo ainda não aberto (rename/code action) abrem a aba com `savedContent = null` (conteúdo em disco desconhecido ⇒ sempre dirty).

## 4. Avaliação falha (BrokenImport)

- Projeto com `state: "failed"` e sem `sources` mostra na árvore os `.cs` do outline textual e o próprio `.csproj`.
- O painel Problems ganha a seção **Evaluation** com o erro de cada projeto falho (ex.: MSB4019); clicar abre o `.csproj`. A aba Problems conta esses erros.
- Fora do escopo: habilitar Build com avaliação falha.

## Testes

- `tests/e2e/qa-top4.spec.ts` (Playwright, host real, fixtures reais) — um bloco por achado, escrito antes da correção.
- Testes Rust só onde houver mudança no host (nenhuma prevista).
