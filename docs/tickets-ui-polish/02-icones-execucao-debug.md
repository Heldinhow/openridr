# 02: Ícones de execução e depuração

**What to build:** Play, Debug, Stop, Build, Restart, Resume, Pause e Step Over/Into/Out ficam legíveis e coerentes com o conjunto. Play e Stop preenchidos; Play e Debug verdes e Stop vermelho (D4 define se Play/Debug passam a neutros). O Terminal deixa de usar o ícone de Play para "Start shell". Step Over/Into/Out: avaliar candidatos do Lucide (`redo-2`, `arrow-down-to-line`, `arrow-up-from-line`) em captura com sessão de debug real; se reprovarem, usar `debug-step-*` do Codicons (CC-BY-4.0) e registrar a exceção (D8).

**Blocked by:** 01.

**Status:** implementado (commit local; ver RETRO-UI-POLISH).

- [ ] Toolbar: Play, Debug, Build, Settings reconhecíveis a 100% (crop nativo antes/depois).
- [ ] `TerminalPanel.tsx:226` usa ícone de terminal, não de Play.
- [ ] Barra de passos do Debug capturada com **sessão ativa** (não verificada no diagnóstico).
- [ ] Contraste ≥ 3:1 dos ícones de ação sobre `--bg`; foco por anel.
- [ ] Se Codicons entrar: licença no `THIRD-PARTY-NOTICES.md` e decisão D8 em `decisions.md`.
