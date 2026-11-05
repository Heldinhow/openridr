# Evidência — release 0.3.0-alpha.7

Data: 2026-11-05. macOS ARM64 (Apple Silicon), build **release** deste worktree
(`release/0.3.0-alpha.7`, base `dd84990` = merge do PR #23 + os commits da release).
Nota publicada: [RELEASE-ALPHA.md](../RELEASE-ALPHA.md). Processo: [release-signing.md](../release-signing.md).

## Build assinado e notarizado

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)"
bun run tauri build --target aarch64-apple-darwin --bundles app
ditto -c -k --keepParent --sequesterRsrc "$app" /tmp/OpenRidr-alpha7-app.zip
xcrun notarytool submit /tmp/OpenRidr-alpha7-app.zip --keychain-profile notary --wait
xcrun stapler staple "$app"
# DMG montado com hdiutil a partir do .app já grampeado, assinado e notarizado depois
```

| Item | Resultado |
|---|---|
| `openridr` (Rust) | `Finished release profile in 1m 02s` |
| Bundle | `OpenRidr.app` 24,02 MiB, assinado com `Developer ID Application: Helder Santos (K6M85697D4)` |
| Notarização do app | `id b1959d2e-8c10-45a2-afb8-150dcd0d3c8b`, `status: Accepted` |
| Notarização do DMG | `id a8ce9e66-7e0d-4bf7-8844-fa783dd69cc8`, `status: Accepted` |
| DMG | `/tmp/OpenRidr_0.3.0-alpha.7_aarch64.dmg`, 10.620.421 bytes |
| SHA-256 | `2fc75e6c441a6ddf31c313429b76eeef831c443b231d6f4b63c32c7d3a5567bc` |

## Verificação da assinatura (todas passaram)

```
codesign --verify --deep --strict --verbose=2 OpenRidr.app   -> valid on disk / satisfies its DR
codesign -dv --verbose=4 OpenRidr.app                       -> flags=0x10000(runtime), TeamIdentifier=K6M85697D4
spctl --assess --type execute -vv OpenRidr.app              -> accepted, source=Notarized Developer ID
spctl --assess --type open --context context:primary-signature -vv <dmg>
                                                            -> accepted, source=Notarized Developer ID
xcrun stapler validate OpenRidr.app / <dmg>                 -> "The validate action worked!"
CFBundleShortVersionString / CFBundleVersion                -> 0.3.0-alpha.7
file OpenRidr.app/Contents/MacOS/openridr                   -> Mach-O 64-bit executable arm64
codesign -dv OpenRidr.app/Contents/Resources/netcoredbg/netcoredbg
                                                            -> Developer ID + runtime (netcoredbg embutido)
```

As mesmas checagens foram repetidas **no app de dentro do DMG montado**
(`/tmp/OpenRidrDmg/OpenRidr.app`): `codesign --verify --deep --strict` OK, `spctl` = *accepted,
source=Notarized Developer ID*, `stapler validate` OK, versão 0.3.0-alpha.7.

## Testes

| Suíte | Resultado |
|---|---|
| `bun run check` (`tsc --noEmit` + `cargo check --workspace --all-targets`) | exit 0 |
| `bun run build` | exit 0 (JS 1.323,76 kB, CSS 109,61 kB, 3 woff2 = 131,6 kB) |
| `bun test` (unitários) | 21 pass, 0 fail |
| `cargo test --workspace` | 235 (lib) + 14 (debugger) + 78 (host) verdes; **2 falhas conhecidas** |
| E2E `solution-explorer` + `focus-after-navigation` (portas 1450/4347) | **17/17**, exit 0 |

Falhas de `cargo test`, idênticas ao baseline do main e listadas no `AGENTS.md`:

- `file_access_is_confined_to_the_workspace` — `expected ../../../../../../etc/hosts to be
  rejected`, veio `not_found` em vez de `outside_workspace`.
- `evaluation_returns_the_real_msbuild_model_for_every_project` — `not_restored` em vez de
  `restored` (pré-condição de restore, não executada nesta máquina).

A suíte E2E completa não foi rodada nesta release; as falhas conhecidas dela (`find-usages`
intermitente, 2 em `run-configs`, 2 em `tests`, 2 de `ui-layout` que falham também no HEAD) estão
declaradas em [RELEASE-ALPHA.md](../RELEASE-ALPHA.md).

## Smoke no app do DMG (agent-desktop, instância única)

```sh
sh .agents/skills/verify-openridr/scripts/desktop.sh up /tmp/OpenRidrDmg/OpenRidr.app
# bundle 0.3.0-alpha.7, OPENRIDR_HOME isolado, fixture OrderDesk copiada
agent-desktop click "Trust project"
agent-desktop click "Dependencies" (OrderDesk.Tests) / "Packages" / "Frameworks"
agent-desktop click "Build"
agent-desktop click "Program.cs"
sh .agents/skills/verify-openridr/scripts/desktop.sh down
```

Visto no app real, a partir do DMG somente leitura:

1. **Árvore sem chrome de avaliação**: só `OrderDesk · 3 projects`, pastas, projetos e arquivos;
   `MSBuild evaluated`, `Evaluate`, `revision N` e `Project details` não aparecem. O estado está na
   barra de status (`Evaluated · Debug · TFM not selected · revision 2`) com o botão de re-avaliar.
2. **Ícone por tipo e badge de símbolo**: `OrderDesk.Cli` (console), `OrderDesk.Core` (biblioteca),
   `OrderDesk.Tests` (testes), `C# Program.cs`, `C Order.cs`; árvore e aba com o glifo C# neutro.
3. **Dependencies agrupadas**: `Frameworks` → `Microsoft.NETCore.App (net8.0)`, `Packages` →
   `Microsoft.NET.Test.Sdk 17.11.1`, `xunit 2.9.2`, `xunit.runner.visualstudio 2.8.2`.
4. **Build**: `Build succeeded · exit 0`, 0 warnings, 0 erros, 0,55 s, com os dois `.dll` no output.
5. **Editor**: `Program.cs` aberto em JetBrains Mono, breadcrumb `OrderDesk.Cli / Program.cs /
   net8.0`, UI em Inter, status bar com keymap `Rider (macOS)`.
6. Sem segunda instância em nenhum momento (`pgrep -x openridr` = 1 durante o smoke, 0 depois do
   `down`); o DMG foi desmontado no fim.

Capturas: `docs/evidence/release-alpha7/01-dmg-solution-explorer-build.jpg` e
`02-dmg-editor-program-cs.jpg`.

## Não verificado

- Tema claro (o build só distribui o escuro).
- Projeto `web` na árvore (não há fixture web).
- Sessão de debug a partir do DMG nesta release (o netcoredbg embutido foi verificado na alpha.6 e a
  assinatura do binário embutido foi conferida aqui).
- Suíte E2E completa (só os dois specs ligados ao conteúdo desta release rodaram).
- Windows e macOS Intel.
