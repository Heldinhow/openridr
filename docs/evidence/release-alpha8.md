# Evidência — release 0.3.0-alpha.8

Data: 2026-11-05. macOS ARM64 (Apple Silicon), build **release** deste worktree
(`release/0.3.0-alpha.8`, base `0fbd844` = merge do PR #29 no `main` + os commits da release).
Nota publicada: [RELEASE-ALPHA.md](../RELEASE-ALPHA.md). Processo: [release-signing.md](../release-signing.md).

## Build assinado e notarizado

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)"
bun run tauri build --target aarch64-apple-darwin --bundles app
ditto -c -k --keepParent --sequesterRsrc "$app" /tmp/OpenRidr-alpha8-app.zip
xcrun notarytool submit /tmp/OpenRidr-alpha8-app.zip --keychain-profile notary --wait
xcrun stapler staple "$app"
# DMG montado com hdiutil a partir do .app já grampeado, assinado e notarizado depois
```

| Item | Resultado |
|---|---|
| `openridr` (Rust) | `Finished release profile in 1m 12s` |
| Bundle | `OpenRidr.app`, 24,25 MiB (24.832 KiB), assinado com `Developer ID Application: Helder Santos (K6M85697D4)` |
| Notarização do app | `id 3bd4c0e8-cd9b-474e-a3a4-416b80595516`, `status: Accepted` |
| Notarização do DMG | `id 1c3c8c58-ac91-419f-b6e0-269391d8238b`, `status: Accepted` |
| DMG | `/tmp/OpenRidr_0.3.0-alpha.8_aarch64.dmg`, 10.699.101 bytes (10,20 MiB) |
| SHA-256 | `bed67e4347e7ba40d4d5d396a4fb5345842ef0daf1baad9c8f814e2ed7ae4b0a` |

O SHA acima é o do arquivo **depois** do `stapler stitch` do DMG (o `codesign --verify` continua
válido nele). O zip de transporte não é publicado.

## Verificação da assinatura (todas passaram)

```
codesign --verify --deep --strict --verbose=2 OpenRidr.app   -> valid on disk / satisfies its DR
codesign -dv --verbose=4 OpenRidr.app                       -> flags=0x10000(runtime), TeamIdentifier=K6M85697D4
spctl --assess --type execute -vv OpenRidr.app              -> accepted, source=Notarized Developer ID
codesign --verify --verbose=2 <dmg>                         -> valid on disk / satisfies its DR
spctl --assess --type open --context context:primary-signature -vv <dmg>
                                                            -> accepted, source=Notarized Developer ID
xcrun stapler validate OpenRidr.app / <dmg>                 -> "The validate action worked!"
CFBundleShortVersionString / CFBundleVersion                -> 0.3.0-alpha.8
file OpenRidr.app/Contents/MacOS/openridr                   -> Mach-O 64-bit executable arm64
codesign -dv OpenRidr.app/Contents/Resources/netcoredbg/netcoredbg
                                                            -> Developer ID + runtime (netcoredbg embutido)
```

As mesmas checagens foram repetidas **no app de dentro do DMG montado**
(`/tmp/OpenRidrDmg/OpenRidr.app`): `codesign --verify --deep --strict` OK, `spctl` = *accepted,
source=Notarized Developer ID*, `stapler validate` OK, versão 0.3.0-alpha.8. O volume montado tem só
`OpenRidr.app` e o link para `/Applications`.

## Testes

| Suíte | Resultado |
|---|---|
| `bun run check` (`tsc --noEmit` + `cargo check --workspace --all-targets`) | exit 0 |
| `bun run build` | exit 0 (JS 1.357,83 kB, CSS 117,30 kB, woff2 Inter/JetBrains) |
| `bun test tests/unit` | **33 pass, 0 fail** (126 `expect()`) |
| `cargo test --workspace` | **393 pass, 0 fail** — 263 (lib) + 14 (debugger) + 98 (host) + 10 (test_discovery) + 7 (test_run) + 1 (trust_model) |
| E2E (portas 1510/4407) | **36 de 37**, 1 falha conhecida em `tests.spec.ts` (abaixo) |

As duas falhas que a alpha.7 registrava no `cargo test` — `file_access_is_confined_to_the_workspace`
e `evaluation_returns_the_real_msbuild_model_for_every_project` — não aparecem mais: o PR #25
corrigiu a causa ([file-access-confinement.md](file-access-confinement.md)) e a suíte está inteira
verde nesta máquina, com o SDK 8.0.408 no `PATH` e a fixture `OrderDesk` restaurada.

### E2E executado

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet \
  DOTNET_CLI_UI_LANGUAGE=en OPENRIDR_E2E_UI_PORT=1510 OPENRIDR_E2E_HOST_PORT=4407
bunx playwright test tests/e2e/acceptance-fixes.spec.ts tests/e2e/native-context-menu.spec.ts \
  tests/e2e/nuget.spec.ts tests/e2e/setup-gate-layout.spec.ts tests/e2e/solution-explorer.spec.ts \
  tests/e2e/solution-file-icons.spec.ts tests/e2e/tests.spec.ts
```

| Spec | Resultado |
|---|---|
| `acceptance-fixes.spec.ts` (D1, D3, D3b, D4, D5, D6) | 5/5 |
| `native-context-menu.spec.ts` | 3/3 |
| `nuget.spec.ts` | 4/4 |
| `setup-gate-layout.spec.ts` | 2/2 |
| `solution-explorer.spec.ts` | 11/11 |
| `solution-file-icons.spec.ts` | 5/5 |
| `tests.spec.ts` | 6/7 |

A falha é a conhecida do `main`: `tests.spec.ts:317` espera `tests-run-id` casando
`/testrun:\d+:1/` e o host devolve `testrun:65:2 · discovery:65:24 · discovery revision 24 · run
revision 2 · context 98` — o spec assume a **primeira** execução da sessão, e o run é o segundo
(revisão 2). O comportamento do produto está correto; o spec é que fixa a revisão. Está declarada em
[RELEASE-ALPHA.md](../RELEASE-ALPHA.md) junto com as demais falhas conhecidas da suíte completa.

## Smoke no app do DMG (agent-desktop, instância única)

```sh
sh .agents/skills/verify-openridr/scripts/doctor.sh /tmp/OpenRidrDmg/OpenRidr.app   # todos ok
sh .agents/skills/verify-openridr/scripts/desktop.sh up /tmp/OpenRidrDmg/OpenRidr.app
agent-desktop click "button:Trust project"
agent-desktop click "Build"
agent-desktop click "NuGet tool window"
sh .agents/skills/verify-openridr/scripts/desktop.sh down
```

Visto no app real, a partir do DMG somente leitura, com `OPENRIDR_HOME` isolado e cópia da fixture:

1. **Setup antes do trust.** O home isolado nasceu vazio; o app instalou o `csharp-ls` 0.28.0 em
   `$OPENRIDR_HOME/tools/csharp-ls/0.28` (o `toolhome` e os `tmp` do `dotnet tool install` ficaram
   no home isolado) **antes** de mostrar o diálogo de trust. O `.app` montado só teve o aviso de
   sandbox-extension do macOS no log, sem efeito no fluxo.
2. **Welcome com o ícone novo.** O selo `OR` com a moldura colorida aparece no cabeçalho da welcome e
   na toolbar do workspace — é o ícone do #29 no app, não só no repositório.
3. **Árvore com os ícones de arquivo.** `Program.cs`, `Order.cs` e `OrderTests.cs` com o glifo C#,
   `Dependencies` com o ícone próprio, projetos com ícone por tipo, e a barra de status
   `Trusted mode · Analysis runtime 10.0.14 · Project SDK 8.0.408 · Evaluated · Debug · TFM net8.0 ·
   revision 2 · Analysis: Partial coverage`, keymap `Rider (macOS)`.
4. **Tests.** Abas `Explorer / All tests from OrderDesk.Tests / Continuous Testing / Coverage`
   (as duas últimas desabilitadas), contadores `0 ✓ 0 ✗ 0 ⏭ 4`, filtro, `Expand all`/`Collapse all`/
   `Export`, árvore `OrderDesk.Tests → OrderTests → 4 testes`, `Discovered 4 tests · exit 0`,
   `TFM net8.0` e o painel de detalhe com "Select a test or group to see its last result and duration."
5. **NuGet.** `Scope Project/Solution`, `Source All configured sources`, `Prerelease`,
   `Installed`/`Discover`, seções com badge (`Implicitly Installed Packages in OrderDesk.Cli: 0`),
   uma única barra de seleção e os logs reais `dotnet list package · ok · exit 0` /
   `dotnet list package --outdated · ok · exit 0`.
6. **Build.** `Build succeeded · exit 0`, 0 warnings, 0 erros, 1,58 s, com `OrderDesk.Core.dll` e
   `OrderDesk.Cli.dll` no `bin/Debug/net8.0` da cópia.
7. Sem segunda instância em nenhum momento (`pgrep -x openridr` = 1 durante o smoke, 0 depois do
   `down`); o DMG foi desmontado no fim.

Capturas: `docs/evidence/release-alpha8/01-dmg-welcome-trust.jpg`,
`02-dmg-tree-tests.jpg`, `03-dmg-build.jpg` e `04-dmg-nuget.jpg`.

## Não verificado

- Tema claro (o build só distribui o escuro).
- Projeto `web` na árvore (não há fixture web).
- Sessão de debug a partir do DMG nesta release (o netcoredbg embutido foi conferido por assinatura e
  está no bundle; a execução do adapter não faz parte deste smoke).
- Suíte E2E completa (rodei os sete specs ligados ao conteúdo desta release; `find-usages`,
  `run-configs` e `ui-layout` não rodaram aqui e continuam com as falhas conhecidas do `main`).
- Windows e macOS Intel.
