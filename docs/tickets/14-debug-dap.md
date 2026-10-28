# 14: Depurar por DAP com breakpoints, frames e locals

**What to build:** Em plataforma habilitada com adaptador verificado, executar sessão DAP real de netcoredbg usando build Debug e saída gerenciada avaliados, inspecionar pausa e encerrar sem confundir protocolo e processos.

**Blocked by:** 11 — Compilar contexto e navegar erros no output; 13 — Instalar debugger com confirmação e gates de plataforma.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Exigir build Debug bem-sucedido do contexto/revisão identificados e TargetPath atual; não passar `dotnet run` como DLL. Trust, asset, versão/plataforma e flag são gates no host e nos atalhos; build falho/adaptador ausente/contexto inválido não lançam.
- [ ] Sessão DAP real usa `--interpreter=vscode`, negocia capacidades/ordem e mostra preparação, rodando, pausada, encerrada, cancelada ou falha. Breakpoint pendente/verificado vem do adaptador; sem símbolos continua pendente. Gutter separa número de linha, breakpoint e linha atual.
- [ ] Pausada: continue, step over, step into, frames e locals do frame selecionado funcionam. Ao continuar/trocar frame/sessão, locals antigos não são atuais; respostas tardias não ressuscitam pausa anterior. Stop e cancelamento têm resultado observado.
- [ ] Preservar log DAP/adaptador/host e output da aplicação; exit ausente mostra `exit unknown`. Sessão encerrada não prova árvore encerrada: verificar processos ou manter remanescentes/falha visíveis, incluindo revogação e troca de workspace.
- [ ] Windows x64 exige smoke DAP próprio completo. Mac Intel sem asset mantém indisponibilidade validada, não muda versão. ARM64 continua experimental OFF por padrão, com opt-in explícito para experimento sem bloquear release e sem prejudicar Run/análise/testes.
- [ ] Para retirar a flag ARM64, exigir prova real Mac ARM64 DAP de launch/breakpoint/stepping/stop **incluindo async e multi-thread**, logs/versões/comandos e processos encerrados. Spike CLI, binário, versão ou opt-in não contam. Sem prova, manter flag/limitação; #256 é risco não reproduzido, não falha universal.
- [ ] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), incluindo teclas Ctrl+D/Cmd+F8 no Mac, Shift+F9/Ctrl+F8 no Windows e F8/F7 em ambos, com gates e foco acessíveis.

## Mapeamento à SPEC

AC-10/18 (saída e falha build), AC-21 (smoke completo condicionado), AC-22 (bloqueios/pendência/locals/exit), AC-23 (gate experimental ARM64), AC-24 (prova Windows e limites Intel), AC-01/07, AC-32/33. SPEC P0.5 e Testing Decisions; sem expandir para attach/watches/debug test.

## Validação

UI desktop → host → DAP real em plataforma apta, breakpoint/continue/steps/frames/locals/stop e processo observado. No Mac ARM64, executar apenas como experimento opt-in; CLI não prova compatibilidade. Boundary DAP/processos: mensagens tardias, desconexão, stop/cancelamento, build falho, símbolos ausentes e exit desconhecido. Registrar cada ambiente/gate não exercitado como pendência; Windows exige sua própria evidência, e ausência de UI em CI não aprova desktop.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
