# 03 — F3: Busca global

**What to build:** Search Everywhere com abas All, Classes, Files, Symbols, Actions e Text, aberta por ⌘T ou Shift Shift, com prévia do arquivo, abertura por Enter, limite declarado e estado honesto quando a análise não está pronta. Inclui os contratos de host para listar arquivos, buscar texto e consultar símbolos.

**Blocked by:** 02

**Status:** done

- [x] ⌘T e Shift Shift (duplo toque em 400 ms) abrem a busca com a última aba e o último texto; Esc fecha e devolve o foco ao editor.
- [x] Tab/⇧Tab alternam as abas; setas navegam; Enter abre o resultado no editor; a prévia mostra caminho e trecho com a ocorrência destacada.
- [x] Files lista arquivos do workspace ignorando `bin/`, `obj/`, `.git/` e `node_modules/`, com cap de 5.000 e truncamento declarado.
- [x] Classes e Symbols usam `workspace/symbol` do csharp-ls via host (cap 500), separam tipo de membro pelo `kind` e mostram "Analysis not ready" com a ação faltante quando trust/análise não estão prontos.
- [x] Actions lista e executa as ações do registro, mantendo o motivo em ações desabilitadas.
- [x] Text busca no workspace sem `ripgrep` (cap 200 arquivos e 2.000 ocorrências, arquivos > 1 MiB ignorados), agrupa por arquivo e declara truncamento.
- [x] Resultado vazio real mostra "No results"; indisponibilidade nunca aparece como lista vazia.
- [x] A busca não executa código do repositório; Files/Text leem arquivos e Symbols só usa sessão LSP autorizada.
- [x] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam; commit local `feat(F3): ...`.
