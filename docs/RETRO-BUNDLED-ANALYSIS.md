# RETRO — análise embutida (csharp-ls + runtime privado)

Onda: 2026-11-05, branch `feat/bundled-analysis-runtime` (base `eb3fe6f`, alpha.8). Spec:
`docs/SPEC-BUNDLED-ANALYSIS.md`; tickets: `docs/tickets-bundled-analysis/`; evidência:
`docs/evidence/bundled-analysis.md`.

## Números

- 3 tickets, 1 commit de implementação + docs. 55 MB de artefatos baixados no build (não
  versionados), 140 MB extraídos no `.app`.
- `.app` 24,25 → **163,29 MiB**; DMG 10.699.101 → **61.774.338 bytes** (+51,0 MiB, ~5,8×).
- Testes: 400 no host (266 + 4 + 14 + 98 + 10 + 7 + 1), todos verdes; nenhum teste novo lento.

## O que funcionou

1. **Provar viabilidade antes de desenhar o bundle.** Extrair o tarball, o payload do nupkg e
   compor um `hostroot` à mão deu `CS0029` real em minutos; sem isso o desenho "DLL + muxer do
   bundle" seria aposta.
2. **Reusar o `hostroot`.** O par SDK externo + runtime 10 já existia; trocar só a **fonte** do
   runtime 10 manteve o resto do comportamento (SDK do projeto manda, csharp-ls não registra
   outro MSBuild default).
3. **Verificar o par, não o instalador.** `sync_bundled` roda `--version` uma vez e publica
   `ready`; nenhum plano, progresso, staging ou download aparece. O caminho de download ficou
   intacto para `tauri dev`.
4. **Sandbox como simulação honesta.** `deny network-outbound (remote ip "*:*")` + raiz só com
   SDK 8 provou setup/análise/debug offline; `deny network-outbound` sozinho quebrava o restore
   (MSBuild usa UNIX socket), então o filtro ficou só em IP.
5. **lsof como prova de identidade.** `ps`/`lsof` mostrando o DLL em `Contents/Resources` e o
   muxer no `hostroot` fecha a pergunta "o bundle é realmente usado?" melhor que qualquer log.

## O que doeu

1. **Entitlements de novo, e por outro motivo.** O runtime ad-hoc com hardened runtime não
   carrega `libhostfxr` ("different Team IDs"); `disable-library-validation` + `allow-jit` no
   muxer resolveu. O erro só aparece na primeira execução, não no `codesign --verify`.
2. **Tamanho.** 140 MB de recursos viram +51 MiB de DMG. Não é otimizável sem cortar runtime
   (arriscado) ou trocar o csharp-ls por self-contained publish (mudaria a resolução de MSBuild).
3. **`tools.plan` como única recusa.** Foi barato deixar `plan/install/retry` falharem com a
   mesma mensagem, mas a UI precisou de um campo (`bundled`) para não mostrar Reinstall e para
   não chamar de "installing" o que é verificação.
4. **Restore "incompleto" na UI.** Editar um arquivo depois do restore deixa o projeto
   `RESTORE INCOMPLETE` — comportamento existente, não desta mudança; confundiu a leitura da
   prova até conferir os artefatos.

## Lições

- **Ferramenta embutida de runtime é um processo filho, não só um arquivo.** Assinatura,
  entitlements e o muxer copiado no `hostroot` são parte do contrato; testar `--version` não
  testa carregamento de dylib sob hardened runtime.
- **Um campo de estado explícito vale mais que inferir.** `bundled` no tool e no runtime evitou
  reescrever a UI em cima de "plan == null".
- **Simulação de ambiente precisa dizer o que ficou de fora.** A máquina ainda tem .NET 10;
  a evidência diz isso em vez de chamar de "Mac limpo".

## Pendência aberta

- **Regressão no E2E `csharp-ls.spec.ts:150`** (Problems após SIGSTOP/SIGCONT): falha nesta
  branch. A sonda host/LSP reproduz o mesmo desfecho na base `eb3fe6f` (servidor vira zombie
  depois de STOP/CONT, sem publicação), então não há evidência de que a branch seja a causa; o
  spec depende de o servidor sobreviver ao sinal. Detalhe em `docs/evidence/bundled-analysis.md`.

## Rodada 2 (revisão independente)

- **Bundle incompleto falha fechado**, não cai no download: `Bundled | Absent | Damaged`, com
  recusa em setup, `tools.plan/install/retry`, `setup.reinstall` e `lsp.start`; runtime externo
  nunca mascara a falta. Fallback de dev preservado.
- **Integridade do par**: fingerprint de `csharp-ls/` + `dotnet/` (entrada, dependência, muxer,
  runtime) invalida o ready; SHA-256 de conteúdo único no log. Custo: metadados por snapshot,
  hash só na verificação.
- **Vendor**: manifest de conteúdo por arquivo (pós-assinatura) decide reutilização; árvore
  danificada é remontada do artefato pinado e reassinada.

## Pendências do Helder

1. **Assinatura/notarização** com Developer ID e `stapler` (exige credencial e Xcode completo).
2. **Quarentena real** (abrir o DMG baixado com `com.apple.quarantine`) e um Mac **de verdade**
   sem runtime .NET 10.
3. **Revisão legal** dos notices (csharp-ls MIT + runtime MIT, com `LICENSE.txt` e
   `ThirdPartyNotices.txt` do tarball dentro do app).
4. **Aceitar o acoplamento**: atualizar csharp-ls ou o runtime agora exige release do app.
