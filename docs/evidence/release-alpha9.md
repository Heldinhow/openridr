# Evidência — release 0.3.0-alpha.9

Data: 2026-11-05. macOS ARM64 (Apple Silicon, macOS 27.2 / build 26B118), build **release** deste
worktree (`release/0.3.0-alpha.9`, base `b5d7964` = merge do PR #36 no `main` + este commit).
Nota publicada: [RELEASE-ALPHA.md](../RELEASE-ALPHA.md). Processo: [release-signing.md](../release-signing.md).

Esta é a **primeira release com o runtime .NET embutido** ([evidence/bundled-analysis.md](bundled-analysis.md)),
então a assinatura e a notarização foram o primeiro passo da release, antes de notas, PR e tag.

## Build assinado e notarizado

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH"
export APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)"
bun run tauri build --target aarch64-apple-darwin --bundles app
ditto -c -k --keepParent --sequesterRsrc "$app" /tmp/OpenRidr-alpha9-app.zip
xcrun notarytool submit /tmp/OpenRidr-alpha9-app.zip --keychain-profile notary --wait
xcrun stapler staple "$app"
# DMG montado com hdiutil a partir do .app já grampeado, assinado e notarizado depois
```

| Item | Resultado |
|---|---|
| `openridr` (Rust) | `Finished release profile [optimized] in 1m 04s` |
| `vendor-analysis.sh` | runtime 10.0.14 e csharp-ls 0.28.0 baixados do artefato pinado, extraídos e assinados com Developer ID + hardened runtime + timestamp |
| Bundle | `OpenRidr.app`, 171.549.682 bytes (163,6 MiB) |
| Notarização do app | `id c528d785-3019-4c6c-9846-02635556f74f`, `status: Accepted` |
| Notarização do DMG | `id 43b9d494-0c0b-4b08-a22a-73b777c622a6`, `status: Accepted` |
| DMG | `/tmp/OpenRidr_0.3.0-alpha.9_aarch64.dmg`, 69.559.207 bytes (66,34 MiB) |
| SHA-256 | `1d2a84a1cde4b4ddb17ac4b3bd4b85ec63ec988160b89391f1445bf52fb52419` |

O SHA acima é o do arquivo **depois** do `stapler stitch` do DMG (o `codesign --verify` continua
válido nele). O zip de transporte não é publicado.

### Cada Mach-O embutido, um por um

Primeira release em que o app carrega um runtime .NET próprio, então a checagem não parou em
`codesign --deep`: os 18 Mach-O do bundle foram inspecionados individualmente com
`codesign -dv --verbose=4` + `codesign -d --entitlements -`.

| Binário | Assinatura | Runtime | Timestamp | Entitlements |
|---|---|---|---|---|
| `Contents/MacOS/openridr` | Developer ID (K6M85697D4) | `flags=0x10000(runtime)` | sim | nenhum |
| `Resources/dotnet/dotnet` (muxer) | Developer ID (K6M85697D4) | sim | sim | `allow-jit`, `disable-library-validation` |
| `Resources/dotnet/host/fxr/10.0.14/libhostfxr.dylib` | Developer ID | sim | sim | nenhum |
| `.../Microsoft.NETCore.App/10.0.14/libhostpolicy.dylib` | Developer ID | sim | sim | nenhum |
| `.../libcoreclr.dylib` | Developer ID | sim | sim | nenhum |
| `.../libclrjit.dylib` | Developer ID | sim | sim | nenhum |
| `.../libclrgc.dylib`, `.../libclrgcexp.dylib` | Developer ID | sim | sim | nenhum |
| `.../libmscordaccore.dylib`, `.../libmscordbi.dylib` | Developer ID | sim | sim | nenhum |
| `.../libSystem.Native`, `libSystem.Globalization.Native`, `libSystem.IO.Compression.Native`, `libSystem.Net.Security.Native`, `libSystem.Security.Cryptography.Native.Apple` (`.dylib`) | Developer ID | sim | sim | nenhum |
| `.../Microsoft.NETCore.App/10.0.14/createdump` | **Microsoft Corporation (UBF8T346G9)** | sim | sim | `disable-library-validation` |
| `Resources/netcoredbg/netcoredbg` | Developer ID (K6M85697D4) | sim | sim | `allow-jit`, `disable-library-validation` |
| `Resources/netcoredbg/libdbgshim.dylib` | Developer ID | sim | sim | nenhum |

17 dos 18 são nosso `Developer ID Application: Helder Santos (K6M85697D4)` com
`flags=0x10000(runtime)` e timestamp; `createdump` vem assinado pela Microsoft no próprio runtime
oficial (Developer ID de terceiro, hardened runtime e timestamp válidos — a Apple aceita nested code
de outro time). O app inteiro passa `codesign --verify --deep --strict`; a notarização do app e do
DMG foi **Accepted** na primeira submissão.

## Verificação da assinatura (todas passaram)

```
codesign --verify --deep --strict --verbose=2 OpenRidr.app   -> valid on disk / satisfies its DR
codesign -dv --verbose=4 OpenRidr.app                       -> flags=0x10000(runtime), TeamIdentifier=K6M85697D4
spctl --assess --type execute -vv OpenRidr.app              -> accepted, source=Notarized Developer ID
xcrun stapler validate OpenRidr.app                         -> "The validate action worked!"
codesign --verify --verbose=2 <dmg>                         -> valid on disk / satisfies its DR
spctl --assess --type open --context context:primary-signature -vv <dmg>
                                                            -> accepted, source=Notarized Developer ID
xcrun stapler validate <dmg>                                -> "The validate action worked!"
CFBundleShortVersionString / CFBundleVersion                -> 0.3.0-alpha.9
file OpenRidr.app/Contents/MacOS/openridr                   -> Mach-O 64-bit executable arm64
```

As mesmas checagens foram repetidas **no app de dentro do DMG montado**
(`/tmp/alpha9-qmnt/OpenRidr.app`): `codesign --verify --deep --strict` OK, `spctl` = *accepted,
source=Notarized Developer ID*, `stapler validate` OK, versão 0.3.0-alpha.9. O volume montado tem só
`OpenRidr.app` e o link para `/Applications`.

## Caminho de download: quarentena simulada

O app foi aberto pelo caminho que um usuário faz de verdade com um DMG baixado:

```sh
xattr -w com.apple.quarantine "0083;<hex-epoch>;Safari;<uuid>" OpenRidr_0.3.0-alpha.9_aarch64.dmg
hdiutil attach OpenRidr_0.3.0-alpha.9_aarch64.dmg -nobrowse -mountpoint /tmp/alpha9-qmnt
ditto /tmp/alpha9-qmnt/OpenRidr.app /tmp/alpha9-qapp/OpenRidr.app    # herda o xattr da imagem
open --env OPENRIDR_HOME=/tmp/alpha9-qstate/home -n -a /tmp/alpha9-qapp/OpenRidr.app \
  --args /tmp/alpha9-qstate/work/OrderDesk/OrderDesk.sln
```

O macOS tratou o bundle como baixado: o processo rodou **translocado**
(`/private/var/folders/…/T/AppTranslocation/…/d/OpenRidr.app`) e apresentou **uma** confirmação de
primeira abertura — "'OpenRidr.app' é um app baixado da internet. Tem certeza de que deseja abri-lo?
… A Apple buscou por software malicioso e não detectou nenhum." (`present code-evaluation prompt`
no log do `CoreServicesUIAgent`; `XProtect suppress first launch warning: true`). Isso é o diálogo
padrão de primeiro lançamento do macOS 15+, não uma recusa de Gatekeeper: o app já estava
`accepted, source=Notarized Developer ID` nas duas avaliações (`spctl` e o próprio prompt).

Respondido **Abrir** (clique físico via agent-desktop), o app abriu e seguiu o fluxo normal.
Captura: `release-alpha9/00-gatekeeper-prompt.jpg`.

> Nota de método: lançar o bundle com quarentena **sem** responder esse diálogo deixa o processo
> parado em `_dyld_start` (96 KB de footprint) — é o kernel esperando a decisão do usuário, não um
> defeito do app. Sem quarentena (cópia via `ditto` da imagem não marcada) o app abre direto. Ambas
> as coisas foram observadas aqui.

## Setup direto, sem download (no app notarizado e translocado)

Fixture `OrderDesk`, `OPENRIDR_HOME` isolado e vazio, **uma** instância:

| Checagem | Resultado |
|---|---|
| Diálogo de setup | **nenhum**: a primeira tela depois da welcome é o prompt de trust |
| `$OPENRIDR_HOME` depois de abrir | `toolhome/`, `hostroot/`, `recent.json`, `trusted.json`, `run-configs/` — **sem** `tools/` |
| `hostroot/` | 128 KiB, composite por symlink: `dotnet` (muxer 125.760 B) + `host/fxr/10.0.14 → <app>/Contents/Resources/dotnet/…` + `shared/Microsoft.NETCore.App/10.0.14 → <app>/…`, mais `sdk/8.0.408` e `shared/Microsoft.NETCore.App/8.0.15` do `~/.dotnet` |
| `hostroot/.openridr-hostroot.json` | `layout=project-sdk-only-runtime10`, `runtimeRoot=<app>/Contents/Resources/dotnet`, `projectVersion=8.0.408`, `hostFxr=10.0.14`, `workerRuntime=8.0.15` |
| Processo do analisador | `hostroot/dotnet <AppTranslocation>/…/OpenRidr.app/Contents/Resources/csharp-ls/CSharpLanguageServer.dll --solution …` |
| Status bar | `Analysis runtime 10.0.14 · Project SDK 8.0.408` · `Evaluated · Debug · TFM net8.0 · revision 2` |

Ou seja: o par embutido executou de dentro do bundle (translocado), sob hardened runtime, **sem
nenhum download** e sem runtime .NET 10 externo. Captura: `release-alpha9/01-dmg-trust.jpg`.

## Análise e build no app do DMG

1. **Trust.** Clicado *Trust project*; workspace abriu com a árvore `OrderDesk · 3 projects`
   (`src` → `OrderDesk.Cli`, `OrderDesk.Core`, `tests` → `OrderDesk.Tests`) e os ícones da árvore do #35.
2. **Diagnóstico ao vivo.** Aberto `Program.cs` e digitado no editor (via agent-desktop) a linha
   `int broken = "not an int";` na linha 6. O `csharp-ls` embutido publicou
   `error CS0029: Cannot implicitly convert type 'string' to 'int'` — a aba Problems foi a `1 error`,
   com `LSP · csharp-ls · session 1 · 1 current · coverage partial`, `6:14`
   `src/OrderDesk.Cli/Program.cs`, e o sublinhado no editor. Captura:
   `release-alpha9/02-dmg-problems-diagnostic.jpg`.
3. **Desfazer apaga o diagnóstico.** Dois `⌘Z` no editor: a barra volta a `0 current`, a aba a
   `Problems` sem contagem e o contador de erros da status bar desaparece.
4. **Build.** *Build startup 'OrderDesk.Cli' (⌘F9)* → **`Build succeeded · exit 0`**,
   `0 build problems · origin Build · dotnet`, `0 Warning(s)`, `0 Error(s)`, `Time Elapsed 00:00:01.41`,
   com `OrderDesk.Core -> …/bin/Debug/net8.0/OrderDesk.Core.dll` e
   `OrderDesk.Cli -> …/bin/Debug/net8.0/OrderDesk.Cli.dll`. Conferido no disco:
   `OrderDesk.Cli.dll` (5.120 B) e `OrderDesk.Core.dll` (4.608 B) em `bin/Debug/net8.0`.
   Captura: `release-alpha9/03-dmg-build.jpg`.

Uma única instância em todo o smoke (`pgrep -x openridr` = 1), fechada ao final
(`agent-desktop close-app OpenRidr` = `process_instance_exited`, `pgrep` vazio); os dois volumes
foram desmontados e nenhum `AppTranslocation` sobrou.

## Testes

| Suíte | Resultado |
|---|---|
| `bun run check` (`tsc --noEmit` + `cargo check --workspace --all-targets`) | exit 0 (66 crates, 9,45 s) |
| `bun test tests/unit` | **35 pass, 0 fail** (134 `expect()`, 6 arquivos) |
| `cargo test -p openridr-host` | **406 pass, 0 fail** — 268 lib + 8 bundled_analysis + 14 debugger + 98 host + 10 test_discovery + 7 test_run + 1 trust_model |
| Verificação de UI | app desktop real (agent-desktop), sem Playwright |

O `cargo test` desta release foi a suíte única pedida pela tarefa; `bun run build` também rodou como
parte do `tauri build` (o `beforeBuildCommand` chama `bun run build`).

## Não verificado

- **Suíte E2E / Playwright**: não rodada nesta release por decisão explícita do Helder (verificação
  de UI só no app desktop real). `csharp-ls.spec.ts:150` (Problems depois de SIGSTOP/SIGCONT) segue
  aberto desde o #32, sem causa demonstrada.
- **Sessão de debug a partir do DMG**: o `netcoredbg` embutido foi conferido por assinatura e está no
  bundle; o adapter não foi executado neste smoke.
- Tema claro (o build só distribui o escuro), projeto `web` na árvore (sem fixture web).
- Windows e macOS Intel.
