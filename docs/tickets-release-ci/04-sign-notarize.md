# 04 — Developer ID, auditoria de Mach-O, notarização e grampo

**Fonte:** `docs/SPEC-release-ci.md` §“Assinatura e auditoria de Mach-O”, §“Notarização e grampo” e
§“Verificação do artefato”. **Bloqueado por:** 03. **Status:** ready-for-agent.

## Objetivo

O caminho completo que o CI executa no push da tag: construir com Developer ID, provar que **todos**
os Mach-O embutidos estão assinados com a nossa identidade e hardened runtime (com a exceção nominal
do `createdump` da Microsoft), notarizar e grampear o `.app`, montar/assinar/notarizar/grampear o
DMG e verificar o artefato com a mesma lista do `docs/release-signing.md`.

## Pontos de código

- Modo sem flag em `scripts/release-macos.sh`: preflight estrito → build com
  `APPLE_SIGNING_IDENTITY` → auditoria → `ditto` do `.app` para `$RUNNER_TEMP` → `notarytool submit
  --wait --output-format json --no-progress` com `--key/--key-id/--issuer` → `stapler staple` e
  `validate` → `hdiutil` → `codesign` no DMG → notarizar/grampear/validar o DMG → verificação.
- Auditoria de Mach-O: detecta por magic number (`cffaedfe`/`cefaedfe`/`cafebabe`); por arquivo exige
  `codesign --verify --strict`, `flags=0x10000(runtime)`, `Timestamp=` e
  `Authority=Developer ID Application: <identidade>`. Única exceção: basename `createdump`, que exige
  `Authority=Developer ID Application: Microsoft Corporation (UBF8T346G9)`; qualquer outro
  `Authority` divergente falha nomeando o arquivo. Dylibs não têm `TeamIdentifier`: compara-se
  `Authority`. A auditoria também roda sozinha (`--audit <app>`) para poder ser provada sem
  notarizar.
- Em rejeição da Apple: baixar e imprimir `notarytool log <id>` com as mesmas credenciais e falhar,
  sem publicar nem deixar DMG parcial no caminho de publicação.
- `scripts/release-selfcheck.sh`: roda `--audit` contra `/Applications/OpenRidr.app` quando existir
  (leitura apenas) e prova que a última release real passa e que a mesma auditoria reprova com
  identidade trocada (a lista de Mach-O encontrados é impressa, inclusive `createdump`).

## Critérios de aceite

- [ ] `--audit /Applications/OpenRidr.app` lista os 18 Mach-O, aceita `createdump` pela Microsoft e
      passa na identidade `Developer ID Application: Helder Santos (K6M85697D4)`.
- [ ] `--audit` com identidade trocada (`APPLE_SIGNING_IDENTITY=Developer ID Application: Outra
      Pessoa (ZZZZZZZZZZ)`) falha nomeando os arquivos com a nossa assinatura.
- [ ] Caminho completo aborta antes de `notarytool` se algum Mach-O estiver ad-hoc (stub): mensagem
      aponta arquivo e motivo.
- [ ] `notarytool` recebe `--issuer` só quando `APPLE_API_ISSUER` existe; credenciais nunca vão para
      argv imprimido (o `.p8` é path, não conteúdo).
- [ ] Verificação final cobre app, app montado e DMG (codesign, spctl, stapler, versão, sha256).
- [ ] Não verificado nesta sessão: assinatura Developer ID/notarização reais (sem credenciais) —
      registrado no ticket e em `docs/evidence/release-ci.md`.
