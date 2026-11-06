# Evidência — diff do Git com arquivo inteiro, modo unificado e hash do commit (#48)

Data: 2026-11-06. Alvo verificado: macOS arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

## O que foi provado

### Host (`crates/host/src/git.rs`)

`cargo test -p openridr-host --lib -- git::` → 18 testes passando, incluindo os 6 novos de `file_diff`:

- textos completos dos dois lados + `hunks` (`-U0`) + commit `HEAD` de um repositório real criado no próprio teste;
- base = **index** no diff unstaged com conteúdo staged e worktree diferentes (o caso que separa o hash do commit do lado esquerdo) e base = `HEAD` no staged;
- arquivo untracked (sem base) e arquivo deletado (sem lado atual) com o texto do `HEAD`;
- binário (`0x00`) e arquivo acima de 1 MiB → `full: false` com `fullReason` (`binary` / `too_large`) e o patch como fallback;
- UTF-16 com BOM decodificado como texto (não binário) e repositório sem commits (`commit: null`);
- caminho fora do repositório → `invalid_state`.

`git.diff` continua com o comportamento antigo: o teste existente (`diff_shows_worktree_staged_and_untracked_content`) passou sem mudança.

`cargo test -p openridr-host --test host -- trust_grant_needs_an_open_workspace_and_gates_every_direct_call` → passa: `git.fileDiff` entrou em `EXECUTABLE_METHODS` e é negado sem trust.

### UI (unidade + E2E)

- `bun test tests/unit/commitDiff.test.ts` → 11 testes passando: montagem do arquivo inteiro (modificação no meio de 10 linhas, dois hunks com contexto entre eles, adicionado, deletado, inserção no topo e no EOF, arquivo vazio), recusa de hunk fora do arquivo/contagem que não fecha (`null` → fallback), projeção unificada (par modificado vira `-`/`+`, numeração nova, diferença → linha do documento) e a preferência de modo (só `unified` vale; o resto volta a `split`).
- `bunx playwright test tests/e2e/git.spec.ts` → **9 passed**: inclui três specs novos (arquivo inteiro + conectores + linha do `HEAD` com o hash de `git rev-parse`, adicionado/deletado com rótulos e conteúdo, binário caindo no patch com a explicação) e o modo unificado (um editor, `+`/`-`, contagem igual, sem conectores, escolha sobrevivendo a fechar/reabrir e troca de arquivo).

## App Tauri real (`agent-desktop`)

Bundle de debug construído do HEAD desta sessão (`src-tauri/target/debug/bundle/macos/OpenRidr.app`, `0.3.0-alpha.11`), `OPENRIDR_HOME` isolado em `/tmp`, workspace aberto por argv sobre uma cópia em `/tmp` de um projeto C# real (sem histórico do repo original; `bin/`/`obj/` fora). Uma instância por vez; o app foi fechado ao final.

- **Split, arquivo inteiro:** o diff de um arquivo modificado mostrou o arquivo todo (linhas de contexto antes e depois da mudança), rótulos `Index` / `Worktree`, `1 difference` e a nota "Showing the full file from Git." O conector aparece no gutter entre os painéis: bloco azul na linha modificada e metade verde na linha adicionada (verificado por pixel no bundle renderizado, não só pela árvore de acessibilidade).
- **Hash do commit:** a linha `HEAD eaa8564 · Baseline for issue 48 verification` aparece entre os rótulos e a nota, com o hash completo no `title`. O hash de blob não é mais exibido como revisão.
- **Unificado:** o toggle `Split | Unified` troca para um único editor; o documento mostrou `-` na linha removida (sem número) e `+` nas linhas adicionadas com o número novo; a contagem continuou `1 difference` e não há conectores. Depois de fechar e reabrir o diff, `Unified` permaneceu ativo (`localStorage`, chave `openridr.commitDiffMode.v1`).
- **Bug encontrado e corrigido nesta verificação:** a linha do commit não era pintada. Causa: `overflow: hidden` no `<p>` zera o `min-height: auto` do flex item e ele encolhia a zero dentro da coluna; a nota seguinte ocupava o lugar. Correção: `flex: 0 0 auto` nas faixas de cabeçalho do diff (`toolbar`, `labels`, `commit`, `note`), com `toBeVisible()` novo no E2E para não regredir.

## Não verificado

- Windows e macOS Intel (não há build nem execução).
- No app real não foram abertos, um por um, os casos de arquivo **adicionado**, **deletado** e **binário** (só E2E). O caminho de arquivo grande (>1 MiB / >30.000 linhas) só foi provado no host, não na tela.
- O fallback `unreadable` (arquivo existe mas não pode ser lido) não tem fixture: é um ramo defensivo sem prova de ponta a ponta.
- O diff a partir do Git log continua fora de escopo (issue #101).

## Rodada 1 da revisão independente

| Achado | Correção | Prova |
|---|---|---|
| Symlink fora do repo lido pela worktree (e pelo `--no-index`) | confinamento do diretório real antes de ler/diffar; symlink é objeto link (alvo em texto) | `git::tests::file_diff_refuses_symlinks_that_leave_the_repository`: diretório symlink recusado (`failed`, sem patch), arquivo sob diretório trocado por symlink recusado sem o segredo externo, symlink solto mostra o alvo (não o conteúdo) |
| Captura sem limite (`Command::output`, `fs::read`) | `spawn_bounded` com teto e kill, stderr drenado em thread, blob por tamanho declarado, teste de 6.000 linhas reescritas | `patch_capture_is_bounded_and_the_ranges_still_align_the_full_texts`, `full_file_sides_stop_at_the_limit_without_collecting_the_file` |
| Contexto com mesmo tamanho e texto diferente era aceito | comparação par a par nos gaps e na cauda → `null` + `unaligned` no painel | `tests/unit/commitDiff.test.ts`: "refuses context lines whose two texts disagree" (antes/entre/depois/zero hunks) |
| UTF-16 caía em `unaligned` | ranges do `git diff --no-index` sobre o texto decodificado em temporários privados 0600 com `Drop` | `hunk_ranges_are_read_from_the_decoded_text_for_utf16` (LE e BE, staged e unstaged), `decoded_diff_temporaries_are_private_and_removed` (0600 e limpeza) |
| Erro de blob virava ausência | `rev-parse --verify --quiet` separa ausência de erro; conteúdo só depois de `cat-file -s`; vazio + ilegível = `failed` | `blob_reads_tell_absence_apart_from_a_read_error` (ausência em HEAD e no index, blob corrompido → `unreadable`, estado `failed` sem inventar adição) |

Limites desta rodada: app Tauri real não foi reaberto (correções sem mudança visível; a verificação anterior de split/unificado/hash continua); Windows/macOS Intel não executados; a janela TOCTOU entre confinamento e uso não foi fechada (declarada).
