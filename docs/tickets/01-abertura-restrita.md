# 01: Abrir solução ou projeto em modo restrito

**What to build:** Abrir a UI desktop mínima com identidade OpenRidr, escolher solução ou projeto SDK-style e editar texto sem executar conteúdo do workspace. Incluir nesta fatia o mínimo shell/base auditada necessário à demo; não criar ticket horizontal de bootstrap nem presumir integração OpenIdea existente.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

**Category:** enhancement

- [x] Auditar a revisão/interfaces/licenças da base efetivamente reutilizada antes de incorporá-la; preservar avisos aplicáveis, sem código/branding proprietário. Entregar uma janela desktop funcional, não apenas os wireframes. *(Nenhum código OpenIdea reutilizado; janela Tauri real criada em 1440×900. Captura e direção da janela nativa limitadas por TCC — ver evidência.)*
- [x] Abrir uma solução ou projeto, apenas um workspace por vez; múltiplas soluções/contextos exigem escolha explícita. Mostrar caminho completo, `Restricted mode`, trust desmarcado e `Open text only` sem ações executáveis no repositório.
- [x] Exibir contorno **textual**, nunca rotulado como árvore avaliada. Ler solução/projeto/global.json não aciona MSBuild, restore, LSP, tarefas, build/run/debug, testes ou shell; limites também existem na fronteira host, sem confiar só em botões desabilitados.
- [x] Abrir/editar/salvar texto inclusive sem solução, SDK ou LSP; árvore à esquerda, editor flexível, toolbar, painel inferior, abas/breadcrumb e modificado identificáveis. Falhas de leitura/escrita preservam buffer e mostram erro/log.
- [x] Desde esta fatia, fechar/substituir buffer ou trocar/fechar workspace com edição exige escolha explícita. Detectar conflito de disco antes de salvar; não sobrescrever silenciosamente nem descartar conteúdo. Um fluxo mínimo seguro pode bloquear a operação mantendo ambas as versões recuperáveis; 07 completa comparar/recarregar/preservar.
- [x] Aplicar [gates G1–G5](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket) pertinentes desde agora, especialmente UI inglesa, foco/teclado, motivos de bloqueio, logs locais e proteção dos dados. Comandos futuros indisponíveis não executam por atalho ou chamada direta.

## Evidência

Implementação, comandos, resultados e limitações por ambiente: [docs/evidence/01-abertura-restrita.md](../evidence/01-abertura-restrita.md). Windows não foi executado; a janela desktop foi criada mas não dirigida (TCC de Screen Recording/Assistive Access), e a UI foi exercitada no Chrome real via ponte HTTP, que chama o mesmo host.

## Mapeamento à SPEC

AC-01 (abertura/restrição/edição), AC-08 (parcial: abertura, escolha e contorno textual; árvore real em 03), AC-11 (parcial: edição e segurança mínima; ciclo completo em 07), AC-32 (shell/editor desta fatia), AC-33 (evidência por ambiente desta fatia), AC-34 (parcial: base incorporada). SPEC P0.1, P0.2, P0.3, Apresentação, Base e fronteiras.

## Validação

Na UI desktop real do Mac, abrir solução/projeto e caso ambíguo, editar/salvar e tentar fechar/conflitar; provar nenhuma execução do workspace por observação de processos e chamadas diretas negativas ao host. Testar na boundary paths com espaços, cancelamento da escolha, falha de escrita e conflito sem perda de dados. Registrar versões/revisões/logs e evidência ou limitação Windows/arquiteturas conforme G5; desenho não conta como E2E.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e validado em macOS 27.2 arm64. Evidência em [docs/evidence/01-abertura-restrita.md](../evidence/01-abertura-restrita.md). Windows não executado; desktop criado mas não dirigido por limitação de TCC; nenhum trust concedido (ticket 02).
