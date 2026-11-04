# RETRO — netcoredbg embutido

Onda: 2026-11-04, branch `fix/e2e-alpha5`. Spec: `docs/SPEC-EMBED-NETCOREDBG.md`; ticket: `docs/tickets-embed-netcoredbg/01-embed-netcoredbg.md`; evidência: `docs/evidence/embed-netcoredbg.md`.

## Números

- 1 ticket, 1 commit de implementação (`f804cd8`) + documentos; 3,39 MB de zip versionado, 9,14 MiB extraídos.
- DMG: **5.671.389 → 10.454.440 bytes** (+4,56 MiB, +84,3%); app release 14,68 → 23,85 MiB.
- Assinatura/notarização: app e DMG aceitos, grampeados, `spctl` Notarized Developer ID; binário aninhado Developer ID + hardened + timestamp.
- Prova real: Debug a partir do bundle de debug com home vazia e do DMG montado somente leitura; `tools/` vazio nas duas.

## O que funcionou

1. **Zip oficial + hash no build.** Versionar o artefato e falhar o build no hash errado deixou o pin auditável e reproduzível; o teste com zip adulterado foi barato.
2. **Assinar o binário antes de copiar.** O script de vendor assina netcoredbg e dbgshim com Developer ID/hardened; o Tauri copia e a assinatura sobrevive. `codesign --deep --strict` passou de primeira no app de release.
3. **Provar entitlements por eliminação.** Sem a matriz (sem entitlement / só um / os dois) o app seria notarizado e quebraria no primeiro Debug. O smoke DAP com o binário hardened foi o teste que decidiu.
4. **Testar o DMG montado, não só o bundle.** O risco "read-only/Contents/Resources" era real na proposta; rodar do volume somente leitura com home vazia fechou a dúvida sem tocar `/Applications`.
5. **UI diz o que é.** "Bundled with OpenRidr", versão, SHA-256 e caminho; sem progresso/retry de download. O usuário não vê um plano que não existe.

## O que doeu

1. **Estimativa de DMG otimista.** A proposta estimou ~8,6 MiB; o UDZO/HFS+ real deu 9,97 MiB. Medir com o método real, não com o zip, foi o que expôs a diferença.
2. **Entitlements não são opcionais.** O binário oficial é ad-hoc; com hardened runtime ele precisa de `allow-jit` e `disable-library-validation`. Isso precisa entrar no release e no notice, não num "se falhar, vemos depois".
3. **Cache do vendor script.** O marker (hash + identidade + entitlements) evita reassinar a cada build, mas a primeira execução com identidade diferente reassina a árvore; documentar isso evita confusão ao alternar debug/release.
4. **Debug bundle engana no tamanho.** 53 MiB com símbolos; a comparação honesta é o app de release (23,85 MiB) e o DMG.
5. **Notarização é lenta e externa.** Duas submissões (app e DMG) com espera; sem o perfil `notary`, a prova não fecha. O fluxo precisa continuar documentado em `release-signing.md`.

## Lições

- **Ferramenta embutida = parte do artefato assinado.** Trate binário aninhado como código do app: pin, hash no build, assinatura, entitlements, notice e teste de execução no volume final.
- **Meça o DMG com `hdiutil` real**, não com o tamanho do zip; a diferença foi ~1,2 MiB.
- **Teste cedo sob hardened runtime.** O código só roda depois de assinar; o entitlement descoberto no fim teria custado uma release quebrada.
- **Leia a licença antes de embutir.** O zip não traz LICENSE; o texto MIT e os notices das DLLs precisam ser publicados com o app.

## Pendências do Helder

1. **macOS Intel**: o release 3.2.0-1092 não publica osx-x64; se Intel voltar ao escopo, é preciso outro asset/versão.
2. **Quarentena/translocation**: a prova foi `spctl`/stapler + DMG read-only; uma abertura real com `com.apple.quarantine` ainda não foi feita.
3. **Revisão legal do `THIRD-PARTY-NOTICES.md`** (MIT do netcoredbg + Roslyn/dbgshim) antes de publicar.
4. **Atualização do adaptador** agora depende de release do app; aceitar esse acoplamento.
