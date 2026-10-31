# Ticket 19 — Evidência visual e E2E

- Ambiente: macOS 27.2 arm64, Chrome 155.0.8059.85 headed, viewport Playwright 1440×900, `slowMo: 300`, SDK 8.0.408; fixture OrderDesk real e host HTTP de desenvolvimento.
- `bun run build`: passou (`tsc --noEmit` + Vite build).
- E2E visual focado: `tests/e2e/run.spec.ts:80`, `tests/e2e/tests.spec.ts:266` e `tests/e2e/debugger.spec.ts:241`; 3/3 passaram no Chrome headed. Cobriu saída Run na área visível do painel, resultados de teste e frames/locals durante pausa real DAP. O Run de curta duração pode avançar de `running` para `exited` entre polls; o teste aceita ambos sem alterar o estado reportado.
- Screenshots capturados e inspecionados, não versionados: `/private/var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T/opencode/ticket19-evidence/{run-output,tests-results,debug-paused}.png`.
- Testes unitários: não há lógica de host/unidade nova nesta mudança de apresentação; não aplicável. Não rodei a suíte E2E inteira.
- Não verificados: Windows x64 e macOS Intel. Nenhuma mudança de estado/contrato do host; testids existentes preservados.
