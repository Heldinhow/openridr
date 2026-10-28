# Evidência — Ticket 13: instalar debugger com confirmação e gates de plataforma

Implementação local em 28/10/2026. Fonte normativa: [ticket 13](../tickets/13-instalacao-debugger.md), [SPEC](../SPEC.md) P0.1/P0.5, “Avaliação e ferramentas”, “Licenças” e Testing Decisions, e gates G1–G5 do [índice de tickets](../tickets/README.md). Nenhum asset oficial foi baixado ou executado nesta máquina; o caminho real está bloqueado por falta de adaptador verificado.

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Versão aprovada | netcoredbg `3.2.0-1092`, release oficial `Samsung/netcoredbg` |
| Fonte de metadata | `https://api.github.com/repos/Samsung/netcoredbg/releases/tags/3.2.0-1092` (28/10/2026) |
| Destino | `<app home>/tools/netcoredbg/3.2.0-1092`, staging `3.2.0-1092.staging` |
| Base de código | `fd4e78f` (ticket 12) + esta fatia |
| Novas dependências | `ureq 2.12` (rustls) e `zip 2` (deflate) |

Log de ambiente e metadata: [`logs/13-release-metadata.log`](logs/13-release-metadata.log).

## Metadata oficial consultada (saída real)

```text
$ curl -sS https://api.github.com/repos/Samsung/netcoredbg/releases/tags/3.2.0-1092
3.2.0-1092 3.2.0-1092 2026-06-25T14:40:46Z prerelease=False draft=False
netcoredbg-linux-amd64.tar.gz	3510717	sha256:080eb3b2d2152465f599d3b33d1ee6e747794e11cc0a3773ec689f5e5f2c5afa
netcoredbg-linux-arm64.tar.gz	3424271	sha256:065ff49badec8a695dbea2de6ab6a330c774a191e426a217ab8cc05250627ccb
netcoredbg-osx-arm64.zip	3389504	sha256:f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea
netcoredbg-win64.zip	3524161	sha256:3c410a45fa502415203a94fcb88654af65bf8e3dac158a5527a722e7a6b9274a

$ curl -sS .../3.2.0-1092/README.md | grep -i arm64
113:*Note: the MacOS arm64 build (M1) is community supported and may not work as expected, as well as some tests possibly failing.*
```

Fatos usados pelo host: **existe** asset `osx-arm64` (com SHA-256 publicado), **não existe** asset `osx-x64`, e o build macOS arm64 é declarado *community supported* pelo próprio upstream. O SHA-256 do plano é o `digest` do GitHub, não um hash calculado pelo OpenRidr; assinatura/notarização não são verificadas nem alegadas. O layout interno do zip oficial não foi inspecionado.

## Resultado real nesta máquina: bloqueado, nada baixado

Estado inicial no Toolchain e no workspace (`13-debugger-plan.json`):

```text
platform osx-arm64 · availability unverified · installable false
adapterState experimental_disabled (sem opt-in) / unavailable (com opt-in)
asset netcoredbg-osx-arm64.zip · sha256 f4fa33b3…4e59ea · 3389504 bytes
adapterMessage "Apple Silicon debugger: experimental · disabled. Unavailable on macOS arm64:
  no verified compatible netcoredbg 3.2.0-1092 adapter. The official release publishes this
  macOS arm64 build as community supported (upstream README: "may not work as expected");
  OpenRidr has no verified compatible adapter or DAP evidence for it (ticket 14). Opting in
  only acknowledges the experiment; it does not install the adapter and does not prove DAP."
gateReason = a mesma mensagem (botão Debug desabilitado)
dapProven false
```

Chamada direta ao host com trust, consentimento e plano válido (`13-debugger-refused.json`):

```text
$ POST /call {"method":"debug.plan","params":{}}
plan.state planned-less: blockedReason preenchido, destination …/tools/netcoredbg/3.2.0-1092
$ POST /call {"method":"debug.install","params":{"planId":"fnv1a64:fce245ebd43aac02","confirm":true}}
{"ok":false,"error":{"code":"unsupported_platform",
 "message":"Installation of netcoredbg 3.2.0-1092 is not available on macos/aarch64: …
  Nothing was downloaded."}}
tools/netcoredbg existe? False
debug.log sem "GET" (nenhum download); adapterState unavailable · installable false
```

O opt-in experimental (`debug.flag {enabled:true, confirm:true}`) muda apenas `appleSiliconExperimental` e o texto; não instala, não baixa e não altera `installable`. Flags e plano são independentes de trust: `debug.plan`/`debug.state`/`debug.cancel`/`debug.flag` não exigem trust; `debug.install`/`debug.retry` exigem (chamadas antes de trust devolvem `trust_required`, testado).

## O que foi construído

- **Metadata e plano** (`crates/host/src/debugger.rs`): versão aprovada, fonte oficial, tabela com apenas os quatro assets publicados (nome, tamanho, `digest` do GitHub, arquitetura, suporte, disponibilidade), plano com OS/arch, destino/staging, operação, permissões, integridade, limites, `blockedReason`, instalação existente e `planId` FNV-1a dos campos materiais. `debug.plan` só lê o disco (não executa nem baixa), e mudança material entre plano e confirmação devolve `confirmation_required` sem download.
- **Gate de plataforma**: Mac ARM64 = `experimental_disabled`/`unavailable` (asset apenas *community supported*, sem prova DAP); Mac Intel = `unavailable` (sem `osx-x64` na release); Linux = `unavailable` (`.tar.gz` não suportado, fora do P0); Windows x64 = `available` (caminho de instalação existe, não executado aqui); qualquer outra combinação = `unavailable`. Nunca há substituição de versão/asset.
- **Download/extração/verificação**: `ureq` com redirects manuais, HTTPS e hosts oficiais (`github.com`, `objects.githubusercontent.com`, `release-assets.githubusercontent.com`), no máximo 5 saltos; hash SHA-256 calculado durante a cópia e comparado ao plano; `zip` com `enclosed_name()`, recusa symlink/modo especial, limite de 200 entradas e 200 MiB, restauração de modo Unix; `--version` no executável extraído precisa conter `3.2.0` **antes** da promoção; instalação válida existente é reutilizada sem download; staging parcial/cancelado/reprovado nunca vira `ready`; progresso só em bytes observados.
- **Ciclo e recuperação**: `downloading → installing (extração) → verifying → ready`, com `installing` observável por ≥300 ms; falha por origem, hash, corrupção, extração ou versão mantém log e erro; `debug.cancel` (sem trust) descarta o parcial e nunca promove; `debug.retry` revalida o plano confirmado; troca/revogação de workspace sela o registro (`sealed`, `staleReason`).
- **UI**: card no Toolchain (setup) e faixa de avisos do workspace, com estado, versão/fonte, asset/availability, mensagem real por plataforma, flag Apple Silicon (checkbox + botão explícito), plano completo, `blockedReason`, log, progresso e SHA/versão verificada. `toolbar-debug` fica desabilitado e o motivo aparece no `title`; a aba Debug repete a limitação. Análise, Run, build e editor continuam funcionando.

## Testes de fronteira (servidor local + zip controlado)

`crates/host/tests/debugger.rs` sobe um servidor HTTP determinístico em `127.0.0.1:0` e usa zips controlados cujo executável é um script rotulado `NET Core debugger 3.2.0-1 (controlled test artifact, Release)`. **Nenhum teste baixa o asset oficial.** Seam: `OPENRIDR_ENABLE_PROBE=1` + `OPENRIDR_DEBUG_PLATFORM=windows/x86_64` + `OPENRIDR_DEBUG_ASSET_URL` + `OPENRIDR_DEBUG_ASSET_SHA256`; o plano marca `asset.testOverride: true`.

```text
$ cargo test -p openridr-host
running 82 tests   (unit; 8 novos do debugger)
test result: ok. 82 passed; 0 failed; 0 ignored
running 8 tests    (boundary do debugger)
test result: ok. 8 passed; 0 failed; 0 ignored
running 49 tests   (boundary dos tickets 01–12)
test result: ok. 49 passed; 0 failed; 0 ignored

$ cargo test -p openridr-host --test debugger a_controlled_install -- --nocapture
controlled netcoredbg install: states ["downloading", "installing", "verifying", "ready"],
  verifiedVersion String("3.2.0"),
  sha256 String("cf028bf7b6fbdfda333b360023f31793b9cbde18ae64607e582479027fb36825"),
  server requests 1
test result: ok. 1 passed; 0 failed
```

Logs: [`13-cargo-test.log`](logs/13-cargo-test.log), [`13-debugger-states.log`](logs/13-debugger-states.log). O SHA-256 acima é do **script controlado** do harness, não do executável oficial.

Casos exercitados em `debugger.rs`:

| Caso | Resultado observado |
|---|---|
| Mac ARM64 real | `experimental_disabled` sem opt-in, `unavailable` com opt-in, `installable:false`, `unsupported_platform` e nenhum diretório criado |
| Trust/consentimento/plano | `trust_required` antes de trust; `confirmation_required` sem `confirm` e com plano adulterado; nenhuma requisição HTTP no plano |
| Plano trocado após confirmação | `confirmation_required` com `details.plan` novo; zero requisições; nada criado |
| Instalação controlada | estados observados `["downloading","installing","verifying","ready"]`, `verifiedVersion 3.2.0`, SHA-256 do executável, log durável, destino promovido, staging removido |
| Reuso | plano com `existing: present`; confirmar reutiliza (`reused:true`), SHA igual, **nenhuma** nova requisição |
| Hash errado | `failed`, “SHA-256 mismatch”, staging/destino ausentes, nada promovido |
| Versão errada | `failed`, “Verification failed”, log com a saída `3.1.1`, destino ausente |
| Zip corrompido | `failed`, “not a readable zip”, destino ausente |
| Traversal / absoluto / symlink | `failed` com “escapes the extraction directory” / “symlinks are not extracted”; nenhum arquivo fora do staging |
| Origem inválida | `failed` antes de qualquer download (`example.invalid`) |
| Redirect inválido | 302 para `https://example.invalid/...` recusado após o primeiro hop; 1 requisição |
| Cancelamento | estado `cancelled` sem promoção, staging ausente, log “Cancellation requested”; cancelar de novo → `invalid_state`; cancelar um plano antes de trust funciona |
| Retry | após cancelar, plano trocado exige `confirmation_required`; novo plano chega a `ready` com `== attempt 2 ==` |
| Gates diretos | `debug.install`/`debug.retry`/`debug.start` devolvem `trust_required` antes de trust; `debug.plan`/`debug.state` são leitura |

## Testes E2E de UI (Playwright + Chrome)

```text
$ bun run test:e2e
  ✓ macOS arm64 blocks the debug adapter honestly and the flag installs nothing (1.7s)
  ✓ host gates consent, trust and plan identity before any download (1.6s)
  ✓ … 36 testes dos tickets 01–12 continuam verdes
  38 passed (3.9m)
```

Log: [`13-playwright.log`](logs/13-playwright.log). Capturas do mesmo build servido ao Chrome: [`screenshots/13-debugger-plan.png`](screenshots/13-debugger-plan.png) (card, plano completo, limites, bloqueio) e [`screenshots/13-debugger-flag.png`](screenshots/13-debugger-flag.png) (opt-in experimental ligado, ainda `unavailable`). JSONs: [`13-debugger-plan.json`](logs/13-debugger-plan.json), [`13-debugger-gates.json`](logs/13-debugger-gates.json), [`13-debugger-refused.json`](logs/13-debugger-refused.json).

```text
$ bun run build
✓ built in 2.23s

$ bun run tauri build --debug --no-bundle
Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.48s
Built application at: /Users/helder/Projetos/openridr/src-tauri/target/debug/openridr
```

Logs: [`13-build.log`](logs/13-build.log), [`13-tauri-build.log`](logs/13-tauri-build.log). Portas 4317/1420 ficaram sem processos ao final da suíte.

## Cobertura do ticket e dos gates

| Item / gate | Resultado | Evidência |
|---|---|---|
| Plano com versão/fonte/URL/asset/OS/arch/destino/operação/permissões; sem asset/origem/consentimento não baixa; sem troca silenciosa | Plano completo com SHA publicado; `unsupported_platform` no Mac ARM64; `confirmation_required` para plano trocado; zero downloads | `13-debugger-plan.json`, `13-debugger-refused.json`, cargo tests |
| G3 origem/redirect/destino/extração/integridade/limites/sem global/elevação/reconfirmação | Hosts HTTPS allowlist, redirects manuais, destino no app home, zip sem traversal/symlink/absoluto com caps, SHA-256 comparado, assinatura/notarização declaradas não verificadas | `debugger.rs`, cargo tests |
| Estados/logs/cancel/retry/parcial não pronta/ferramenta válida preservada/progresso medido/sem DAP | Estados reais no teste controlado, log durável, cancel/retry, reuso sem download, bytes medidos, `dapProven:false` | cargo tests, `13-debugger-plan.json` |
| Mac ARM64 experimental OFF, Debug bloqueado, Run independente, opt-in separado | `experimental_disabled` → `unavailable` após opt-in; `installable:false`; `toolbar-debug` desabilitado com motivo; Run/análise/editor visíveis | E2E 1, capturas |
| Mac Intel/Windows/Linux | Intel e Linux `unavailable` no gate (sem executar essas máquinas); Windows `available` mas não instalado/testado; nenhuma substituição | cargo unit `gate_states_are_honest_per_platform`; limitações abaixo |
| Host aplica consentimento/plataforma/flag/trust em chamadas diretas; G1–G5; MIT e UI acessível | `debug.install`/`debug.retry` atrás de `require_trust`; consentimento e plataforma no host; flag sem trust; controles nativos com nome/motivo; MIT registrado | E2E 2, cargo tests, esta página |
| AC-04/05/06/22/23/24 (fatia), AC-01/07, AC-32/33/34 | Plano/instalação/gates/flag/limites e UI acessível; DAP e auditoria de release ficam em 14/18 | tabela acima |

## Limitações e pendências honestas

- **Nenhuma instalação/launch real do netcoredbg foi executada**, nem no Mac ARM64 (bloqueado por falta de adaptador verificado) nem em Mac Intel/Windows. O zip oficial nunca foi baixado nesta máquina; o caminho de download/extração/verificação foi provado com artefato controlado.
- **Windows x64**: o asset oficial `netcoredbg-win64.zip` existe e o host permite a instalação nessa plataforma, mas nada foi executado aqui; o layout interno do zip, o `--version` real e o smoke DAP são pendências de 14/18.
- **macOS Intel**: a release 3.2.0-1092 não publica `osx-x64`; o gate/plano estão provados em teste de host, mas não houve execução em máquina Intel.
- **DAP**: `ready` prova apenas executável e versão (`--version` contém `3.2.0`). Não há launch, breakpoint, stepping, stop, async nem multi-thread; a flag ARM64 só pode ser retirada com a prova do ticket 14. O relato upstream #256 continua risco não reproduzido.
- **Integridade**: SHA-256 vem do metadata do GitHub; não há assinatura de código/notarização verificada nem hash oficial publicado fora do GitHub. Os hosts de redirect do GitHub não foram exercitados contra a release real (o teste usa 302 local), então o allowlist de redirect é política, não prova de campo.
- **Layout do archive**: o host localiza exatamente um `netcoredbg`/`netcoredbg.exe` em qualquer profundidade do staging; o layout real do zip oficial não foi inspecionado. Se o archive oficial tiver mais de um executável com esse nome, a verificação falha em vez de adivinhar.
- **Linux** fora do P0: assets `.tar.gz` são metadata-only (`unsupported_archive`); o host não chama `tar` externo.
- **Teste controlado**: o executável do zip de teste é um script rotulado e roda apenas no harness; `OPENRIDR_ENABLE_PROBE=1` é exigido para o seam de plataforma/URL/hash.
- **Licença**: netcoredbg é MIT (`LICENSE` da tag 3.2.0-1092, “Copyright (c) 2017 Samsung Electronics Co., LTD”); nenhum código-fonte foi incorporado e nenhum binário é redistribuído nesta fatia. A auditoria de distribuição de assets permanece no ticket 18; nada aqui é parecer jurídico.
- **E2E** usa Chrome real contra Vite + host-http, com `OPENRIDR_HOME` isolado; a janela Tauri nativa não foi capturada (limitação de TCC já registrada). O host E2E não baixa o asset oficial.
