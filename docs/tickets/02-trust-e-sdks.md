# 02: Confiar no workspace e separar SDKs

**What to build:** Tornar explícita a decisão de confiança e mostrar host de análise SDK 10, SDK do projeto, global.json/rollForward e TFM como informações diferentes. Permitir revogar confiança sem esconder processos nem impedir edição quando faltam pré-requisitos.

**Blocked by:** 01 — Abrir solução ou projeto em modo restrito.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Inventariar ferramentas/SDKs globais em contexto neutro fora do workspace; somente depois de `Trust and inspect project` resolver SDK do projeto obedecendo global.json/rollForward. Não tomar SDK 10 como substituto de SDK fixado.
- [x] Host valida trust, workspace e contexto em toda solicitação executável, inclusive chamadas diretas/atalhos; requisições sem trust são negadas sem processo iniciado. Expor o gate aos consumidores futuros sem inventar uma API interna nesta documentação. *(Gate único `require_trust` + `ContextToken`/`accept_result`; testado por dispatch direto e HTTP.)*
- [x] SDK host ausente e SDK projeto ausente têm mensagens próprias, instruções oficiais e nova detecção. Não instalar SDK, alterar global.json/TFM ou migrar projeto; abrir/editar/salvar continua possível. Inventário não significa LSP carregado.
- [x] Revogar trust bloqueia novas execuções imediatamente, encerra serviços do workspace e exige decisão explícita para término de processos ativos. Manter lista/estado dos remanescentes e da confirmação pendente, sem declarar limpeza antes de observá-la. *(Nenhum serviço LSP/restore existe nesta fatia; a invalidação de contexto é o encerramento disponível, e processos controlados provam terminate/keep/awaiting.)*
- [x] Troca/fechamento de workspace protege edições e encerra ou relata processos; autorização não vaza para outro workspace nem é recuperada por resultado tardio. Trust não confirma instalação nem habilita experimento.
- [x] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket): decisões e erros ingleses acessíveis, logs sem segredos, evidência por ambiente e execução com argumentos separados/cwd explícito.

## Evidência

Implementação, comandos, resultados, capturas e limitações: [docs/evidence/02-trust-e-sdks.md](../evidence/02-trust-e-sdks.md). Windows não executado; processos provados com child controlado (probe de desenvolvimento), sem PTY; LSP/MSBuild ficam em 03–06.

## Mapeamento à SPEC

AC-01 (host-enforcement), AC-02 (inventário, SDKs separados, regras e ausência), AC-07 (revogação e processos), AC-16 (parcial: distinção de pré-requisitos; coexistência real em 05), AC-32/33 (superfície/evidências da fatia). SPEC P0.1 e Estado, erros e processos.

## Validação

UI real: confiar, redetectar SDK ausente e revogar. Boundary host: tentar execução antes/depois de revogar, durante confirmação pendente e após troca de workspace; observar negação e processos. Verificar inventário fora do repo e resolução de projeto fixado sem reescrita. Usar processo controlado na boundary para término/remanescente sem depender de tickets posteriores; consumidores reais repetirão os negativos. Registrar limites Mac/Windows sem instalar SDK nesta tarefa documental.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e validado em macOS 27.2 arm64 (host 10.0.100, projeto 8.0.408). Evidência em [docs/evidence/02-trust-e-sdks.md](../evidence/02-trust-e-sdks.md). Windows não executado; nenhum serviço real de LSP/restore existe nesta fatia.
