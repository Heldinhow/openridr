# 33 — Symbols: item fora do workspace não é truncamento

- [x] Raiz: `crates/host/src/lib.rs` (`workspace_symbol_items`) — `truncated` passa a espelhar só o teto do parse (`list.truncated`); símbolo filtrado por estar fora do workspace não conta. Antes: `list.truncated || items.len() < list.items.len()` gerava "truncated by the host (0 returned of 1 total)" ao buscar `Program`.
- [x] Teste unitário `symbols_outside_the_workspace_are_filtered_not_truncated`: red com a lógica antiga, green com a nova; truncamento real preservado.
- [x] Callers da mesma lógica revisados: referências (`parsed.total > targets.len()`) e document symbols (`parsed.total > count`) comparam com a contagem do parse, não com filtro por workspace; rejeições já são contadas à parte. Sem mudança.
- Texto da UI (`FindAction.tsx`) inalterado: só aparece com truncamento real.

Não verificado: busca real no `.app` (a cargo do native-proof-4).
