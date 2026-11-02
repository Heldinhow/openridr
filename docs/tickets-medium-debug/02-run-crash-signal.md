# 02 — Crash do Run: sinal, destaque e pilha clicável (d8)

**Fonte:** `docs/SPEC-MEDIUM-DEBUG.md` §2. **Bloqueado por:** 01 (normalização de caminho). **Status:** ready-for-agent.

## Objetivo

Exceção não tratada (SIGABRT no macOS) deixa de ser "Run exited · exit unknown" neutro: mostra
`Run crashed · SIGABRT` em erro e a linha `at ... in arquivo:line N` do log abre o arquivo.

## Pontos de código

- `crates/host/src/process.rs`: `Entry`/`ProcessStatus` ganham `signal: Option<i32>`; em
  `refresh()` usar `std::os::unix::process::ExitStatusExt::signal()` (cfg unix) quando
  `status.code()` é `None`; `view()` publica `signal`. PTY (`ExitSlot`) fica como está.
- `crates/host/src/run.rs`: `RunView.signal: Option<String>` com o nome curto (`SIGABRT` etc.);
  `observe`/`complete` copiam do `ProcessStatus`. Função de nome com fallback `SIG<n>`.
- `src/host/client.ts`: `ProcessInfo.signal: number | null`; `Run.signal: string | null`.
- `src/BuildPanel.tsx` `runStateLabel`: `failed` → "Run failed to start";
  `exited` + signal → `Run crashed · ${signal}`; senão o texto atual.
- `src/RunPanel.tsx`: `run-state` com classe `error` quando `failed`, signal ou exit ≠ 0;
  `run-exit` mostra `signal SIGABRT`; `run-log` divide o texto em linhas e transforma
  `^\s*at .+ in (.+):line (\d+)$` em `<button data-testid="run-log-frame-N">` que chama
  `onOpenFrame(path, line)`; o resto continua texto (textContent do `<pre>` inalterado).
- `src/WorkspaceView.tsx`: passa `onOpenFrame={(path, line) => void navigateTo(path, { line: line - 1, character: 0 })}`.

## Teste primeiro

- Unidade Rust em `process.rs`: `signal_name(6) == "SIGABRT"`, desconhecido vira `SIG<n>`.
- Nova fixture `fixtures/CrashApp` (console net8.0, `global.json` 8.0.408, `throw new InvalidOperationException`).
- `tests/e2e/run.spec.ts`: abrir/confiar/restaurar CrashApp, `build-action`, `run-action`;
  `run-state` contém "Run crashed" e "SIGABRT" com classe `error`; `run-log-frame-0` visível,
  clique abre `tab-Program.cs` com cursor na linha do `throw`.

## Aceite

- [x] `cargo test -p openridr-host` cobre o nome do sinal (sem novas falhas).
- [x] E2E prova crash, destaque, sinal e clique na pilha.
- [x] Exit 0 continua neutro; sem sinal/código continua "exit unknown".
- [x] `bun run check` verde. Commit `fix(run): report crash signal and clickable stack`.

Ambiente macOS arm64: a exceção não tratada termina com sinal 6 (`SIGABRT`, exit 134). O E2E
afirma `Run crashed · SIGABRT` pela regex `SIG[A-Z]+`; `run-exit` mostra `signal SIGABRT`.
