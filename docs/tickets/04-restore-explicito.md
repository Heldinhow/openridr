# 04: Restaurar explicitamente e recarregar contexto

**What to build:** Solicitar restore como ação consciente, acompanhar logs/resultado e recarregar o contexto avaliado depois, preservando edição e estado parcial quando falhar. Fornecer aos consumidores posteriores contexto atualizado sem permitir restore escondido.

**Blocked by:** 03 — Avaliar MSBuild e mostrar a árvore real.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Oferecer ação explícita de restore com workspace/projeto/contexto e comando visíveis, autorizada pelo host apenas enquanto trusted. Abrir, avaliar ou carregar contexto não inicia restore silencioso.
- [x] Apresentar andamento real, stdout/stderr, resultado, erro, cancelamento e retry explícito; falha/restore incompleto preserva edição e sinaliza dependências/análise parciais, nunca sucesso por falta de diagnóstico.
- [x] Sucesso invalida avaliação e TargetPath anteriores e permite recarregar o modelo real; mudança de configuração/TFM, projeto ou imports também invalida. Durante recarga não reutilizar saída antiga como atual; resposta tardia de restore não reativa outro contexto.
- [x] Estabelecer e testar na boundary a política usada por build/run/test: comandos e pré-condições impedem restore implícito; se necessário informar restore pendente e oferecer a ação explícita, sem contornar a decisão. Cada consumidor posterior valida a política com sua ferramenta real.
- [x] Cancelamento/revogação distinguem pedido de término, processo observado e exit conhecido/unknown; manter logs e remanescentes visíveis, sem registrar credenciais de feeds nem upload automático.
- [x] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com controles acessíveis e SDK/global.json preservados.

## Mapeamento à SPEC

AC-03 (restore explícito, falha e política; comprovação build/run/test também em 11/12/16/17), AC-09 (restore parcial), AC-10 (invalidação/recarga), AC-15 (parcial: restore não garante análise global), AC-01/07 e AC-32/33 na fatia. SPEC P0.1/P0.2 e Estado, erros e processos.

## Validação

UI real Mac: fixture sem restore → solicitar → sucesso/recarga; falha controlada → log → retry, com edição disponível. Boundary: cancelamento, revogação, troca de contexto durante restore e import/TargetPath alterados; verificar nenhum processo implícito e nenhuma resposta obsoleta aplicada. Usar ferramentas reais e registrar comando/SDK/revisão/log/exit; limites Windows ficam explícitos até prova própria.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.

## Evidência

Implementação, comandos, saídas reais de sucesso/falha/cancelamento, timestamps de assets, testes Rust/Playwright e limitações por ambiente: [docs/evidence/04-restore-explicito.md](../evidence/04-restore-explicito.md). Verificado em macOS 27.2 arm64 com SDK de projeto 8.0.408 e host 10.0.100; Windows x64 e macOS Intel não executados. Consumidores reais de build/run/debug/testes ficam nos tickets 11/12/16/17 e devem chamar a mesma política `restore_required`.
