# Retro — debug avançado: hit count e watches (2026-11-06)

Frente da issue #47, tickets #94 (hit count), #95 (watches) e #96 (verificação no app real).
Spec: [SPEC-DEBUG-ADVANCED](SPEC-DEBUG-ADVANCED.md). Evidência:
[evidence/debug-advanced.md](evidence/debug-advanced.md).

## O que mudou

- **Hit count** passou a existir de verdade. O adaptador netcoredbg 3.2.0-1092 ignora
  `hitCondition` (medido: com `hitCondition: "3"` parou nas 5 iterações), então o host conta as
  paradas, casa o frame do topo com o breakpoint e se auto-continua até a N-ésima. Condição filtra
  antes da contagem; Pause manual, passo e exceção não contam; o contador zera no launch e ao
  redefinir o alvo; depois do alvo o breakpoint não para mais e o log fica silencioso.
- **Watches** viraram estado do host: sobrevivem ao Restart/novo Debug, somem ao fechar/trocar o
  workspace e são avaliados contra a thread/frame selecionados, sem manter resultado velho na
  troca.
- **Prova**: E2E novo com host próprio e netcoredbg real (7/7) e app Tauri real via
  `agent-desktop` (condição, hit count com Restart, Pause manual, watch seguindo a thread), com sete
  capturas no bundle final.

## Lições

- **"O adaptador não anuncia" não é o mesmo que "o adaptador recusa".** A decisão de 2026-10-30
  dizia que hit count não entrava porque a capability não era anunciada; medindo o comportamento,
  o netcoredbg **aceita a propriedade e ignora em silêncio**. A decisão foi reaberta com o probe.
  Para qualquer recurso dependente de capability, prove o comportamento, não só o flag.
- **Estado adiado precisa de porta de entrada no host.** Manter `stopped` em `running` para não
  publicar pausa de hit count quebrou `current()` (que só aceitava `paused`/`preparing`): o
  `load_pause` retornava cedo e a sessão ficava presa em `Running`. O E2E pegou; a lição é que
  `current()`/`is_active()` e afins precisam ser revistos junto com qualquer estado novo.
- **Concorrência de auto-continue não é "só count".** Sem prender a retomada a `(session, stop_seq)`
  um auto-continue atrasado pode continuar uma pausa mais nova — inclusive a pausa manual do
  usuário. O `resume` agora recebe a geração esperada; a revisão em subagente achou isso que o E2E
  não acharia.
- **Resultado de watch pertence a uma seleção, não só a uma pausa.** `watch_stop_seq` sozinho
  deixava o valor do frame anterior aparecer depois de trocar de thread/frame. Guardar o
  `frame_id` da avaliação resolveu o valor velho e a resposta fora de ordem.
- **Spec de feature preso ao fluxo de instalação quebra quando a entrega muda.** O
  `debugger.spec.ts` do HEAD ainda copia o netcoredbg para `tools/` e chama `debug.install`; com o
  adaptador embutido o host responde "nothing to install" e o teste de gates espera
  `experimental_disabled`. O E2E desta frente subiu host próprio apontando o bundle para o
  adaptador real, sem depender daquele spec. Pendência registrada, fora deste escopo.
- **Teste de Restart tem que mudar algo observável.** Esperar `Paused` de novo aceitava o mesmo
  pause anterior: o teste passava sem observar o restart. Agora o teste arma hit count 3 antes do
  Restart e exige sessão maior **e** watch em `2` (antes o valor era 0/3).
- **Experiência no app real achou um bug de UI que o E2E não vê:** as abas do painel Debug
  "Console" e "Debug Output" estão com os rótulos trocados em relação ao conteúdo (`dap-output` no
  Console, `dap-log` no Debug Output). Não corrigido aqui; registrado na evidência.
- **`agent-desktop` é suficiente para este fluxo, com atalhos.** Cmd+L (go to line), Cmd+F8
  (breakpoint), Ctrl+D (debug) e os campos por `aria-label` funcionaram; `set-value` em input
  React não aplica (precisa `focus` + `type --headed`), e elementos fora da viewport aparecem no
  `find` mas não no `snapshot` — usar `find` para o ref.
- **O `pnpm`/bun não roda sozinho em worktree nova**: `bun install` antes de `tsc`/Playwright,
  como o AGENTS.md já diz. E o app de outra sessão na mesma máquina bloqueia a verificação real
  (instância única): esperar, não matar.

## Pendências

- `tests/e2e/debugger.spec.ts` continua quebrado no HEAD pelo adaptador embutido (gates e
  `prepareDebugCopy`); merece ticket próprio.
- Rótulos Console/Debug Output trocados (bug de UI fora do escopo).
- Cobertura que falta: troca de **frame** (não thread) nos watches, Stop + novo Debug separado do
  Restart, limpeza ao fechar/trocar workspace, Pause manual durante auto-continue automatizado, e
  stdout do debuggee observado no app real (o tab certo não foi lido por causa do rótulo).
- Windows e macOS Intel não executados; não são declarados suportados.
- `cargo test --workspace` teve uma falha intermitente em
  `run_start_reports_restore_required_when_assets_are_invalidated_after_a_build` na primeira
  execução; passou isolada e na repetição completa (98/98). Registrada como flake, não regressão.

## Rodada de revisão

- **Alto**: auto-continue podia retomar uma pausa mais nova → `resume` recebe `stop_seq` esperado e
  confere sob o lock que incrementa a geração.
- **Médio**: watch mostrava valor do frame anterior → `watch_frame_id` + descarte de resposta cujo
  frame não é o selecionado.
- **Médio**: pausa intermediária publicada → `stopped` com hit count fica em `running` até o
  `stackTrace` decidir; `current()` aceita esse estado adiado.
- **Médio**: teste de Restart não provava nada → sessão maior + valor `2` após hit count 3.
- **Baixos**: contador `u64` (saturação em `u32::MAX`), log silencioso após o alvo, E2E de condição
  falsa.
- Resultado: `cargo test --workspace` verde (com `OPENRIDR_ENABLE_PROBE=1`), `bun run check`,
  `bun run build`, `bun test tests/unit` 39/39 e o spec novo 7/7.
