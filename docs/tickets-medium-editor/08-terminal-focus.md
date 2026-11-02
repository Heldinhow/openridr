# 08 — b10: terminal recebe foco ao abrir

Status: ready-for-agent
Achado: QA-BUGHUNT-2026-11-02 §10.

## Aceite

- Montar a aba Terminal coloca o foco no textarea do xterm.
- Quando o shell passa a `running` (auto-start), o foco volta para o xterm.
- Digitar sem clicar no terminal chega ao PTY; o formulário de linha continua funcionando.

## Teste

E2E: abrir a aba Terminal, esperar `Running`, afirmar `.xterm-helper-textarea` focado, digitar `echo focus-ok` e ver a saída sem clicar no painel.
