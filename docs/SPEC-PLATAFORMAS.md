# SPEC — Releases para macOS Intel e Windows

Issue-mãe: [#54](https://github.com/Heldinhow/openridr/issues/54) — "Releases for macOS Intel and
Windows". Decisões do grill: `docs/decisions.md` (2026-11-07, "Releases para macOS Intel e Windows").
Tickets: issues filhas de #54. Evidência: `docs/evidence/plataformas.md`. Smoke:
`docs/SMOKE-PLATAFORMAS.md`.

## Problem Statement

O OpenRidr só existe como DMG para macOS Apple Silicon. Quem tem um Mac Intel ou um PC Windows não
tem o que instalar — não é uma questão de "rodar mal": não há artefato. A issue-mãe do release
(`docs/release-decision.md`, 2026-10-28) registrou isso como gates G-WIN-CORE, G-INTEL e G-PKG, e
cada ticket de plataforma desde então terminou com "Windows não executado" ou "Mac Intel não
executado" porque não havia onde rodar.

O problema do usuário é direto: **não consigo instalar a IDE na minha máquina**. O problema do
projeto é o inverso: o código já tem ramos Windows em várias fronteiras (término de processo,
terminal, app home, SDK, netcoredbg), mas nenhum deles foi compilado ou executado no Windows, e o
bundle de análise é macOS-arm64-only por construção. Sem um artefato e um alvo de CI, esses ramos
são fé, não produto.

## Solution

Três **release targets**, cada um com o seu artefato e o seu conjunto de recursos:

| Target | Artefato | Runtime .NET privado | netcoredbg |
|---|---|---|---|
| macOS arm64 (existe) | `OpenRidr_<versão>_aarch64.dmg` | `osx-arm64` | `osx-arm64` (community) |
| macOS x64 (novo) | `OpenRidr_<versão>_x64.dmg` | `osx-x64` | **nenhum** (não existe asset) |
| Windows x64 (novo) | `OpenRidr_<versão>_x64-setup.exe` (NSIS) | `win-x64` | `win64` (official) |

O app passa a se reconhecer instalado em qualquer um dos três (resources ao lado do executável no
Windows, `Contents/Resources` no macOS), o par de análise embutido passa a ser validado por
plataforma, e o build escolhe o runtime e o adaptador do target. O Windows sai **sem assinatura**
nesta rodada: o SmartScreen avisa e o usuário precisa de "Mais informações → Executar assim mesmo",
como o Gatekeeper do macOS já é documentado.

A prova de plataforma não vai para o `ci.yml` de todo PR: um workflow separado (`platforms.yml`,
`workflow_dispatch` + noturno) compila e roda os testes do host em `windows-2025` e
`macos-15-intel` e gera os dois instaladores como artifact do run. O release (assinatura,
publicação, notas) fica bloqueado pela #55, que está reescrevendo `scripts/release-macos.sh` e
`.github/workflows/release.yml` agora.

## User Stories

1. Como desenvolvedor com Mac Intel, quero baixar um DMG que instala e abre, para usar a IDE na minha máquina.
2. Como desenvolvedor com Mac Intel, quero que o app me diga que Debug não está disponível nesta arquitetura, para não caçar defeito num adaptador que não existe.
3. Como desenvolvedor com Mac Intel, quero que o resto do núcleo (trust, SDK, árvore, editor, LSP, build, run, testes, Git, terminal) funcione, para a IDE ser útil mesmo sem Debug.
4. Como desenvolvedor Windows, quero um instalador que instala sem pedir administrador, para não depender de TI.
5. Como desenvolvedor Windows, quero abrir uma solução por associação de arquivo ou por argv, para o fluxo ser o mesmo do macOS.
6. Como desenvolvedor Windows, quero que o analisador e o runtime privado venham no instalador, para não instalar nada além do SDK que o projeto pede.
7. Como desenvolvedor Windows, quero um terminal funcional (PowerShell/cmd via ConPTY), para não sair da IDE.
8. Como desenvolvedor Windows, quero que Build, Run e Tests funcionem com o SDK do `global.json`, para o projeto compilar como no Rider.
9. Como desenvolvedor Windows, quero que o Git funcione com o `git` do PATH, para versionar sem ferramenta extra do app.
10. Como desenvolvedor Windows, quero saber que o instalador não é assinado e como passar pelo SmartScreen, para não achar que baixei malware.
11. Como desenvolvedor Windows, quero que o Debug apareça como "não verificado nesta versão", para não confundir ausência de prova com ausência de caminho.
12. Como usuário de qualquer target, quero que o app falhe fechado quando o bundle perde o par de análise, para uma instalação corrompida não cair silenciosamente num download.
13. Como mantenedor, quero que os testes do host rodem no Windows e no Intel no CI, para os ramos `cfg(windows)` deixarem de ser fé.
14. Como mantenedor, quero que o instalador Windows e o DMG x64 saiam de um run de CI, para alguém poder baixar e testar sem uma máquina de build.
15. Como mantenedor, quero que esses jobs fiquem fora dos checks obrigatórios, para não somar minutos de Windows/Intel a cada PR.
16. Como mantenedor, quero que o `ci.yml` continue rodando só macOS arm64 em PR, para o ciclo de PR não mudar de custo.
17. Como mantenedor, quero que o build de um target não contamine o outro no mesmo checkout, para não empacotar um runtime arm64 num app x64.
18. Como mantenedor, quero um doc de smoke com passos e resultado esperado por target, para a verificação ser repetível e honesta.
19. Como mantenedor, quero que o doc diga o que o CI prova e o que só uma máquina real prova, para ninguém apresentar build verde como UI verificada.
20. Como mantenedor, quero que a fatia de release espere a #55, para dois agentes não reescreverem o mesmo script ao mesmo tempo.
21. Como responsável pela segurança, quero que a assinatura do Windows fique explicitamente fora desta rodada e com issue própria, para a decisão de custo/terceiro não ser tomada por omissão.
22. Como responsável pela segurança, quero que o opener de navegador no Windows não passe por shell, para uma URL de launch profile não virar execução de comando.
23. Como responsável pela segurança, quero que o netcoredbg do bundle Windows venha do asset oficial com o SHA-256 publicado, para o adaptador não ser substituído por outro binário.
24. Como usuário, quero que o nome do artefato diga a arquitetura, para não baixar o DMG errado.

## Implementation Decisions

### Alvos e nomes

- macOS arm64 continua `aarch64-apple-darwin` → `OpenRidr_<versão>_aarch64.dmg`.
- macOS x64 é `x86_64-apple-darwin` → `OpenRidr_<versão>_x64.dmg`, **buildado por cross-compile no
  runner arm64** (`macos-15`), não no `macos-15-intel`. O runner Intel existe para rodar os testes
  do host no ambiente do alvo, não para duplicar o build assinado.
- Windows x64 é `x86_64-pc-windows-msvc` → `OpenRidr_<versão>_x64-setup.exe`, NSIS com
  `installMode: currentUser` (sem admin) e WebView2 por `downloadBootstrapper` (o instalador baixa
  se faltar; o runner já tem).

### Recursos por target

- macOS arm64: `resources/dotnet` = runtime `osx-arm64`; `resources/netcoredbg` = `osx-arm64`.
- macOS x64: `resources/dotnet` = runtime `osx-x64`; **`resources/netcoredbg` sai sem binário** (só
  o `netcoredbg.entitlements` versionado). O `vendor-netcoredbg.sh` remove o binário extraído quando
  a arquitetura não é arm64, para um checkout que buildou arm64 antes não levar um binário arm64
  executável para o bundle x64.
- Windows x64: `resources/dotnet` = runtime `win-x64`; `resources/netcoredbg` = `netcoredbg-win64`.
  Nada é assinado no Windows (Authenticode é a issue filha de assinatura).
- csharp-ls 0.28.0 é managed (AnyCPU) e é o mesmo payload nos três targets.

Hashes oficiais usados (SHA-512 do `releases.json` do canal 10.0 para o runtime; SHA-256 do digest
do GitHub para o netcoredbg):

- `dotnet-runtime-10.0.14-osx-x64.tar.gz` — `b13d453e60f3d718743ada5710d0174524cbf16dead74aa62af6cc258bc5f7310e8ec991fa9365f84bf78bb198417ddceb1381acf557ed52ab0300a21b671596`
- `dotnet-runtime-10.0.14-win-x64.zip` — `a176095f1db69b42c81acc60940744078d6bf2d2ac643e418342aa9c81889f4bd13e42f4621b131f9e77e4bc781ad46e375956219df9a4f979031d2a5a8d93cd`
- `netcoredbg-win64.zip` — `3c410a45fa502415203a94fcb88654af65bf8e3dac158a5527a722e7a6b9274a`

### O app instalado em cada plataforma

- `bundled.rs` deixa de ser macOS-arm64-only: `supported()` vale para macOS (arm64 e x86_64) e
  Windows x86_64; `resources_root()` é `Contents/Resources` no macOS e o diretório do executável no
  Windows (é o que o `resource_dir()` do Tauri resolve no Windows); `in_app_bundle()` é "o caminho
  tem `.app`" no macOS e "o executável não está sob `target/`" no Windows — o mesmo critério que o
  host já usa para reconhecer um binário de teste.
- O par de análise continua validado por layout (`CSharpLanguageServer.dll` + deps, muxer, `host/fxr`
  10.x, `shared/Microsoft.NETCore.App` 10.x) e continua **falhando fechado** quando o app está
  instalado e o par está incompleto. O nome do muxer já é `dotnet.exe` no Windows.
- `debugger.rs`: o adaptador embutido passa a ser procurado pela plataforma do target
  (`binary_name(platform)`), não por um `if platform == "osx-arm64"` repetido em três lugares. No
  macOS x64 o diretório existe sem executável, então `bundled_binary()` devolve `None` e o gate
  `unsupported_platform` (que já tem a frase "the 3.2.0-1092 release publishes no osx-x64 asset")
  continua sendo a resposta.
- `sdk.rs`: as raízes de sistema ganham `C:\Program Files\dotnet` no Windows, e a raiz gerenciada
  (`OpenRidr/sdks`) deixa de ser derivada do `HOME` com um caminho de macOS — ela é uma entrada da
  detecção, ao lado de `DOTNET_ROOT`, `~/.dotnet` e `PATH`.
- `browser.rs`: o opener do Windows é `rundll32 url.dll,FileProtocolHandler <url>`, sem `cmd /C` e
  sem shell — a URL vem de um launch profile e não pode virar linha de comando.
- O seletor nativo de arquivo continua só no macOS; no Windows a resposta é a mesma de hoje
  ("type the path instead"), agora sem prometer um seletor que não existe.

### Vendor por plataforma

- `scripts/vendor-analysis.sh` escolhe o runtime por `TAURI_ENV_TARGET_TRIPLE` (fallback
  `uname -m`): `osx-arm64` ou `osx-x64`. O resto (marker por manifest, assinatura com Developer ID,
  entitlements) fica igual.
- `scripts/vendor-netcoredbg.sh` só extrai no macOS arm64; nos outros targets limpa o que houver e
  sai com uma linha explicando que aquele target não tem asset.
- `scripts/vendor-windows.ts` (bun, sem dependência nova) baixa, verifica SHA e extrai runtime
  `win-x64`, csharp-ls e netcoredbg `win64` para `src-tauri/resources`. É idempotente por presença
  do layout (`dotnet.exe`, `host/fxr/10.0.14`, `shared/Microsoft.NETCore.App/10.0.14`), aceita
  `VENDOR_DEST` para ser exercitado fora do bundle, e não assina nada.
- `src-tauri/tauri.windows.conf.json` sobrescreve `bundle.targets` para `["nsis"]` e
  `build.beforeBuildCommand` para `bun run build && bun scripts/vendor-windows.ts`. O
  `tauri.conf.json` principal não muda de forma para o macOS.

### CI de plataforma

- `.github/workflows/platforms.yml`, separado do `ci.yml`: `workflow_dispatch`, agenda noturna e
  push na `main`. Nada dele é required check.
- `rust-windows` (`windows-2025`) e `rust-macos-intel` (`macos-15-intel`): checkout, toolchain do
  `rust-toolchain.toml`, cache, SDK 8.0.408 das fixtures, `dotnet restore` de aquecimento e
  `cargo test -p openridr-host`. O mesmo `DOTNET_CLI_UI_LANGUAGE: en` do `ci.yml`.
- `package-windows` (`windows-2025`) e `package-macos-x64` (`macos-15`): `bun install`, `bun run
  build`, `tauri build` do target, e upload do instalador como artifact do run. Sem assinatura, sem
  publicação, sem tag.
- O `ci.yml` não ganha job novo: PR continua só macOS arm64.

### Release (bloqueado pela #55)

- `scripts/release-macos.sh` ganha `--target` (arm64/x64) e o nome do DMG por arquitetura; o
  preflight deixa de exigir `uname -m` = arm64 e passa a exigir que o target pedido case com o
  artefato.
- `release.yml` ganha o job Windows (build NSIS, sem assinatura) e publica três anexos; as notas
  listam os três e o aviso do SmartScreen.
- Nada disso é implementado enquanto a #55 estiver aberta; os tickets ficam `Blocked by` a #55.

## Testing Decisions

- **O teste de plataforma mais alto é o CI no alvo real.** Compilar e rodar `cargo test -p
  openridr-host` em `windows-2025` e `macos-15-intel` é o que transforma `cfg(windows)` em
  comportamento observado. Teste unitário não substitui isso, e é por isso que o workflow existe.
- **Funções puras continuam testadas onde já são.** `sdk.rs` (ordem das raízes com a raiz gerenciada
  como entrada), `bundled.rs` (layout do runtime, par incompleto, `pair_expected`), `debugger.rs`
  (plano por plataforma, `bundled_binary` por target com o override de debug). Prior art: os testes
  de `bundled.rs` e `debugger.rs` que já existem.
- **O que não dá para testar no Mac não vira teste falso.** `in_app_bundle()` no Windows e o opener
  do Windows são exercitados pelo job Windows (compilação + teste `#[cfg(windows)]`), não por um
  mock que finge ser Windows no macOS.
- **O instalador é verificado pelo que ele é.** `package-windows` instala o NSIS em silêncio
  (`/S`) num diretório temporário e confere que `OpenRidr.exe`, `dotnet/dotnet.exe`,
  `csharp-ls/CSharpLanguageServer.dll` e `netcoredbg/netcoredbg.exe` estão lá. `package-macos-x64`
  monta o DMG e confere `file` = Mach-O x86_64 e o layout. É o mesmo tipo de verificação que o
  `release-selfcheck.sh` já faz com um `.app` de stub.
- **O smoke de UI é documentado, não inventado.** `docs/SMOKE-PLATAFORMAS.md` tem os passos por
  target, o resultado esperado, a seção "o que o CI prova / o que só uma máquina real prova" e o
  aviso do SmartScreen. A jornada de UI completa fica como **não verificado** até alguém rodar numa
  máquina Windows ou Intel.
- **O teste unix-only de `lib.rs` ganha `#[cfg(unix)]`** — hoje ele impede o `cargo test` de
  compilar no Windows, e é a primeira coisa que o job Windows vai encontrar.

## Out of Scope

- Assinatura Authenticode do Windows (issue filha com `needs-triage`; decisão de custo/terceiro).
- Universal binary (`--target universal-apple-darwin`).
- Debug no macOS Intel (sem asset `osx-x64` na versão pinada).
- Debug no Windows como funcionalidade verificada (o caminho existe; a prova é issue própria).
- Seletor nativo de arquivo no Windows.
- Publicação da release, tag, notas e anexos (bloqueado pela #55).
- Linux.
- Windows arm64.
- E2E/Playwright nos alvos novos.
- Mudar o `ci.yml` de PR.

## Further Notes

- **Por que não universal:** os recursos são por arquitetura. Um universal precisaria dos dois
  runtimes (~70 MB cada) e de escolha em runtime, e o DMG já carrega a arquitetura no nome. O custo
  não paga o benefício num alpha.
- **Por que NSIS e não MSI:** NSIS instala por usuário sem admin, é o default do Tauri e permite
  baixar o WebView2 quando falta. MSI (WiX) exigiria decisão de escopo de máquina e assinatura para
  não piorar o SmartScreen.
- **Risco conhecido do Windows sem assinatura:** o SmartScreen bloqueia com "Windows protegeu o seu
  PC" até o usuário clicar em "Mais informações → Executar assim mesmo". Isso é pior que o Gatekeeper
  do macOS e precisa estar nas notas, não escondido no doc de smoke.
- **O que o CI não prova:** que a janela abre, que a UI responde, que o LSP/build/run/testes
  funcionam de verdade no Windows ou no Intel. Build verde é pré-requisito, não prova de produto.
