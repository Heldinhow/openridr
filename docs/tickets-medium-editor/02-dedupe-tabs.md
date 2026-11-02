# 02 — d13: duplo clique não duplica aba

Status: ready-for-agent
Achado: QA-DEVFLOW-2026-11-02 §13.

## Aceite

- Duas chamadas concorrentes de `openFile` para o mesmo caminho produzem uma única aba.
- O clique simples continua abrindo uma aba.

## Teste

E2E: `dblclick` em `Program.cs`; exatamente uma `tab-src/OrderDesk.Cli/Program.cs`, sem `*` duplicado.
