# 01 — d9: restaurar abas, cursor e breakpoints por solução

Status: ready-for-agent
Achado: QA-DEVFLOW-2026-11-02 §9.

## Aceite

- O store por solução (`run-configs`) ganha uma sessão com `tabs[]` (`path`, `line`, `character`), `activePath` e `breakpoints` (`path → linhas`).
- `runConfigs.write`/`setStartup` preservam a sessão; um método novo grava só a sessão.
- Ao abrir um workspace, as abas salvas são lidas do disco e a aba ativa restaura cursor/scroll; breakpoints são reaplicados quando o workspace está Trusted.
- Arquivo que sumiu do disco não impede abrir as outras abas.
- Persistência é por solução canônica, no app home (nunca no repositório).

## Teste

E2E: abrir `Program.cs`, clicar a linha 3, criar breakpoint, fechar e reabrir; uma aba, cursor em `Ln 3` e `.cm-line-breakpoint` de volta. `runConfigs.read` mostra a sessão gravada. Rust: roundtrip e preservação ao salvar configs.

O spec usa host isolado (`OPENRIDR_TRUST_STORE=on` e session store ligado). O host compartilhado da suíte roda com `OPENRIDR_SESSION_STORE=off` para um spec não restaurar as abas do anterior.
