# 05: Instalar csharp-ls com confirmação e carregar análise

**What to build:** Revisar e confirmar instalação local de csharp-ls 0.28.0 pela origem NuGet.org, verificar a ferramenta e carregar análise real em SDK 10, mantendo projeto fixado em SDK 8 independente. Falha ou ausência de SDK mantém texto editável e estados honestos.

**Blocked by:** 04 — Restaurar explicitamente e recarregar contexto.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Antes de baixar/executar, mostrar plano exato csharp-ls 0.28.0, NuGet.org/URL efetiva, SO/arquitetura, destino absoluto local fora do repo, comando/operação e permissões. Cancelar plano não faz nada executável; trust não é confirmação. Mudança material pede nova confirmação.
- [x] Aplicar todas as proteções G3 pertinentes ao instalador NuGet: origem/redirecionamentos, limites de destino e extração sem escape, mecanismos disponíveis de integridade/autenticidade e limites explicitados. Não presumir assinatura/hash disponível, instalar globalmente, elevar permissões implicitamente, substituir versão ou sobrescrever ferramenta válida com parcial.
- [x] Exibir plano pendente, downloading, installing, verifying, ready, cancelled e failed com log persistente, retry e cancelamento real. Percentual apenas medido; parcial/falha não é ready. Verificar executável e versão antes de ready; isso não significa LSP carregado. SDK de sistema não é instalado.
- [x] Após trust e pré-requisitos, lançar csharp-ls com ambiente que realmente separe host SDK 10 de projeto net8/SDK 8.0.408 com rollForward disabled, sem reescrever global.json. SDK ausente e incompatibilidade têm log/ação de redetecção, não migração.
- [x] Exercitar indisponível, carregando, pronta para operações disponíveis, cobertura parcial, aguardando diagnósticos e falha; oferecer log/reinício explícito sem travar editor. Negociar capacidades/ordem LSP, sem contagens/percentuais de arquivos inventados nem garantia de analyzers.
- [x] Revogar trust encerra serviço e impede reinício pelo host; ignorar mensagens tardias de sessão/contexto anterior. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket) desde esta fatia, incluindo auditoria MIT/avisos da ferramenta e acessibilidade.

## Mapeamento à SPEC

AC-02 (prova coexistência), AC-04/05/06 (parcial: instalador csharp-ls completo; debugger independente em 13), AC-09/15 (limites de cobertura), AC-11 (falha mantém edição), AC-16 (carga/falha/log/reinício e SDKs), AC-01/07, AC-32/33 e AC-34 (parcial: ferramenta incorporada/baixada). Não depende de instalação de debugger.

## Validação

UI desktop Mac → host → instalação real da origem aprovada e LSP real na fixture; provar versões e SDKs efetivamente usados. Boundary determinística: recusar consentimento, plano alterado, origem/redirect inválido, corrupção, escape de destino/extração, permissões, cancelamento/falha/retry, versão divergente e preservação da ferramenta válida. Testes controlados complementam smoke oficial; repetir negativos de trust direto, resposta tardia e desconexão. Registrar evidência ou limitação por ambiente; nenhum SDK é instalado automaticamente.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e verificado em macOS 27.2 arm64. O hostroot mantém o **runtime** 10 e apenas o SDK 8.0.408 em `sdk/`; incluir o SDK 10 no mesmo diretório fez o MSBuildLocator registrá-lo como default e gerou `CS0518` no net8. O SDK 10 do sistema continua sendo o host de instalação/execução. Evidência: [docs/evidence/05-csharp-ls.md](../evidence/05-csharp-ls.md). Windows x64 e macOS Intel permanecem não executados.

## Evidência

Plano/cancelamento/confirmação, instalação real com `--version` e SHA-256, hostroot com runtime 10/SDK 8, LSP real com initialize, CS0246, didChange, restart, revogação, testes Rust/Playwright e limitações por ambiente: [docs/evidence/05-csharp-ls.md](../evidence/05-csharp-ls.md). Verificado em macOS 27.2 arm64; Windows x64 e macOS Intel não executados. Problems/obsolescência ficam no ticket 06.
