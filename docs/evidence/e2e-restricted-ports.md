# Evidência — porta restrita do Chrome recusada na hora (issue #258, ticket #261)

Data: 2026-11-07. Ambiente: macOS Apple Silicon, par de portas 2025 (UI) / 4917 (host), app home
derivado da porta da UI (`<tmpdir>/openridr-e2e-home-2025`), `~/.dotnet` no `PATH`.

Spec: [SPEC-E2E-RESTRICTED-PORTS](../SPEC-E2E-RESTRICTED-PORTS.md). Ticket: #261 (filho de #258).

## 1. O erro aparece no começo do run (critério da issue)

`playwright test --list` carrega a config e nada mais: não sobe host, não sobe Vite, não abre navegador.
É o caminho mais curto para provar que a recusa acontece antes de qualquer servidor.

```sh
OPENRIDR_E2E_UI_PORT=1720 bunx playwright test --list
# exit=1 em 0,3 s
# Error: Invalid E2E port: 1720 is blocked by Chrome (net::ERR_UNSAFE_PORT); choose another port

OPENRIDR_E2E_HOST_PORT=1716 bunx playwright test --list
# exit=1 em 0,3 s
# Error: Invalid E2E port: 1719 (hostPort + 3 of 1420/1716) is blocked by Chrome (net::ERR_UNSAFE_PORT); choose another port pair
```

O segundo caso é a faixa derivada: `1716 + 3` é 1719, a porta que o `isolated-host.spec.ts` usa. A
mensagem nomeia a derivada e o par escolhido.

Par legal, mesmo comando, para o controle:

```sh
OPENRIDR_E2E_UI_PORT=2025 OPENRIDR_E2E_HOST_PORT=4917 bunx playwright test --list
# exit=0 em 1,1 s; Total: 312 tests in 64 files
```

Antes desta frente, `1720` passava pelo validador e o run só morria no `page.goto` com
`net::ERR_UNSAFE_PORT` (medido na sessão da #217, par 1720/4617).

## 2. O harness continua subindo com um par legal

```sh
OPENRIDR_E2E_UI_PORT=2025 OPENRIDR_E2E_HOST_PORT=4917 bunx playwright test tests/e2e/environment.spec.ts
# 3 passed (30.1s)
```

Cobre o validador existente (vazio, `0`, `65536`, `1.5`, `1420;echo`, `-1`), o proxy `/__host` recusando
origem estrangeira, o app home derivado da porta e o host usando esse home isolado.

## 3. Teste unitário (roda no CI de PR, job `typescript`)

```sh
bun test tests/unit
# 123 pass, 0 fail, 387 expect() calls, 17 files
```

O arquivo novo (`tests/unit/e2eRestrictedPorts.test.ts`) tem 7 testes: 1719/1720/1723 recusadas com a
mensagem citando Chrome, 1721/1722/1725 aceitas, validação numérica existente intacta, um subprocesso por
variável (`OPENRIDR_E2E_UI_PORT=1720` e `OPENRIDR_E2E_HOST_PORT=1720`) e a faixa derivada (`HOST=1716` →
1719). Falha-antes medido pelo implementer: 3 pass / 4 fail no código anterior à mudança.

## Não verificado

- Nenhuma porta restrita foi exercitada de fato contra o Chrome: a prova é a recusa no load, não o
  `ERR_UNSAFE_PORT` real (que é o comportamento do navegador, não deste repositório).
- Suíte E2E completa (312 testes) e smoke noturno: não rodados; a área é o validador.
- App Tauri real, Windows e macOS Intel: nada aqui é superfície nativa.
