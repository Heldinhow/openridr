# 06 — Links de localização no Build output (G20)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Build output". **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

Um build com erro mostra, no próprio log, `arquivo(linha,coluna)` clicável que abre o arquivo
na posição. O host fornece os spans; a UI não reparseia MSBuild.

## Pontos de código

- `crates/host/src/build.rs`: `parse_line`, `BuildProblem`, `Data`, `view()`.
- `crates/host/src/logbuf.rs`: acesso às linhas retidas e ao aviso de truncamento.
- `src/BuildPanel.tsx`: renderização do `build-log` (hoje `<pre>`).
- `src/host/client.ts`: tipo dos spans no `BuildView`.
- `src/styles.css`: aparência do link dentro do log.

## Comportamento

- O snapshot do build ganha uma lista de links com: índice da linha no log exibido, coluna
  inicial/final, caminho, linha e coluna do código.
- Os links vêm do mesmo parser conservador que já produz `build.problems`; a UI só encontra o
  span e renderiza um botão. Linha não reconhecida continua texto.
- Prefixo `[stderr] ` e o aviso de log truncado não deslocam o link para a linha errada: os
  spans são calculados sobre o texto exibido.
- Clicar abre o arquivo na linha/coluna via `onNavigate` (caminho já validado no host).
- Nenhum link para caminho fora do workspace.

## Teste e verificação

- Rust: log com erro em stdout, erro em stderr, linha irreconhecível e log truncado; afirmar
  índices/colunas e que nenhum link aponta para fora do workspace.
- Playwright: cópia do OrderDesk com erro de sintaxe; build falha; clicar no link do
  `build-log` e conferir a linha no editor.
- App real (agent-desktop): provocar erro, abrir Build, clicar no link e conferir o cursor.

## Aceite

- [ ] Build com erro proposital mostra `arquivo(linha,coluna)` clicável no log.
- [ ] Clicar abre o arquivo na linha e coluna corretas.
- [ ] Os spans vêm do host; a UI não reparseia o log.
- [ ] stderr e log truncado não deslocam links; linha irreconhecível fica texto.
- [ ] Evidência no app real.
