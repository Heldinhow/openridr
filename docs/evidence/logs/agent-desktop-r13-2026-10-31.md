# agent-desktop — rodada r13 (2026-10-31)

Bundle: `4943eba3…450dc` (HEAD de código `d673114`; delta vs `810e70a8…` é só font-size dos popups, 0.72→0.75rem, mais rustfmt em teste). Cópia `/tmp/ponto/app4`. PIDs próprios: 18570 (encerrado/caiu), 21616 (atual). Janela própria mapeada com `list-windows --app OpenRidr`: `w-54307` = PID 18570, `w-54332` = PID 21616. **Nunca tocado:** `w-40683` = PID 32349 (app pessoal do Helder, minimizado) e 32444 (netcoredbg).

## Resultados

| Item | Ação agent-desktop | Resultado observado | Trace/captura |
|---|---|---|---|
| 5 fechar aba suja | `click` em "Close Program.cs" → `click` em "Discard and close" | Diálogo "Close modified file?" (Save / Keep editing / Discard and close); depois barra "Closed src/OrderDesk.Cli/Program.cs; unsaved edits were discarded.", nenhuma aba `.cs`, disco intacto | native-61; `agent-desktop-trace-close-dirty-2026-10-31.jsonl` |
| 7 completion | `focus` + `press ctrl+space` + `type 'order.'` no editor | `press` entregue por CGEventPost (`verified:false`), sem popup; `focus` não moveu o caret; `type` → `ACTION_FAILED` (AXSelectedText, não verificado), buffer inalterado | não publicado (sem popup) |
| 20 limpeza | `clear` no campo + `click` em "Close" do Search Everywhere | diálogo fechou | — |

## Observações do agent-desktop

- Destravou: **fechar aba suja** (o CGEvent não conseguia; o AX acessa o botão de Close da aba e o diálogo de confirmação).
- Não destravou: **completion** e digitação no editor (webview). `type`/`press` no editor não produziram efeito verificável; não usei coordenadas como fallback, conforme a regra.
- Janela do PID 18570 desapareceu da lista de janelas no meio da rodada, sem que eu tenha encerrado o processo. Causa não verificada. Relancei o mesmo bundle (PID 21616) com o mesmo home isolado.
- Erros de uso encontrados e corrigidos na prática: `get`/`is`/`list-surfaces` não aceitam `--window-id` (use a ref qualificada); `snapshot --skeleton` não aceita `--root`.

## Limites

- Completion (item 7) segue não verificada.
- Fechamento de aba suja é parcial no escopo do item 5 (salvar e conflito já tinham prova no SHA `34a87b38…`).
- Windows e macOS Intel não executados.
