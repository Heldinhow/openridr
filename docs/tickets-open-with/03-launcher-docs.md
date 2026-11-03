# 03 — Launcher `openridr`, docs e evidência

**Fonte:** `docs/SPEC-OPEN-WITH.md` §Solution/Testing Decisions.
**Bloqueado por:** 02. **Status:** ready-for-agent.

## Objetivo

`openridr <caminho>` abre uma solution/projeto/pasta no app já aberto (ou abre o app) pelo
LaunchServices. O uso e a instalação manual no PATH ficam documentados; a evidência registra o
que foi verificado de verdade e o que só vale no bundle real.

## Pontos de código

- Novo `scripts/openridr` (POSIX sh, executável):
  - exige exatamente um argumento e caminho existente (`[ -e "$1" ]`);
  - chama `open -a "${OPENRIDR_APP:-OpenRidr}" "$1"`;
  - erro claro em `stderr` e exit diferente de 0 para uso inválido/caminho ausente;
  - não instala nada, não usa a ponte HTTP, não abre `-n`.
- `docs/install-app.md`: seção "Abrir do Finder e do terminal" com **Abrir Com**, `open -a`,
  associações declaradas, cache do LaunchServices e o link manual no PATH
  (`ln -s "$PWD/scripts/openridr" ~/.local/bin/openridr` ou `/usr/local/bin`). Deixar claro que
  o app não faz isso sozinho.
- `docs/implementation-notes.md`: como o caminho chega (argv/`Opened` → UI → `workspace.open`),
  o que `.slnx` cobre e o limite de restore/build com 8.0.408, o comportamento de uma janela e
  o guard de buffer sujo.
- `docs/evidence/03-open-with.md`: comandos e resultados reais (bundle, Info.plist, aberturas
  fechado/aberto, pasta, launcher) + lista explícita do que não foi verificado.

## Teste primeiro

1. `sh -n scripts/openridr` (sintaxe).
2. Self-check com um `open` falso no PATH (diretório temporário): o script repassa `-a`,
   o app (`OPENRIDR_APP` incluso) e o caminho uma única vez; sem argumento ou caminho
   inexistente sai diferente de 0 sem chamar `open`.
3. Rodar o launcher de verdade contra o bundle de debug com `OPENRIDR_HOME` isolado e a
   fixture, com o app fechado e aberto; registrar em `docs/evidence/03-open-with.md`.
4. `bun run check` no fim; reportar qualquer falha nova.

## Aceite

- [x] `scripts/openridr` executa e passa no self-check com `open` falso.
- [x] Documentação de **Abrir Com**, PATH manual e dev bundle em `docs/install-app.md`.
- [x] Notas de implementação com o limite de `.slnx` e o que não foi verificado.
- [x] Evidência real do bundle e das aberturas, sem tocar no app home real.
- [x] `bun run check` verde (fora as falhas pré-existentes listadas na spec).
- [x] Commit local `feat(cli): add openridr launcher and document Open With` só com os
      caminhos deste ticket.
