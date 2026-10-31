# 01 — F1: Casca com tool windows, faixas e slots

**What to build:** a casca deixa de ter cinco painéis fixos numa faixa e passa a ter um registro de tool windows com faixas de ícones, slots esquerdo e inferior, uma única barra de 44 px (projeto, branch, run config, Run/Debug, busca, Settings) e status bar com breadcrumb, posição, encoding e indentação. Nenhuma funcionalidade de build/run/debug/testes muda; é migração de casca.

**Blocked by:** None (can start immediately)

**Status:** done — commit `483364f`; suíte E2E completa 56/56.

- [x] Build, Problems, Debug, Terminal e Tests abrem pelos ícones das faixas e continuam acessíveis por seus controles e atalhos atuais.
- [x] Cada tool window pertence a um slot (esquerdo ou inferior), uma ativa por slot; abrir outra do mesmo slot alterna, sem split, detach ou reordenar.
- [x] Clicar no ícone ativo oculta a tool window; ⇧Esc oculta a ativa; o foco volta ao editor.
- [x] Slots redimensionam por ponteiro e por setas no divisor; tamanhos e tool windows ativas persistem após reload e valores inválidos são corrigidos.
- [x] A barra é única, com 44 px: projeto (nome + popup de soluções recentes), branch (desabilitado, "Git not connected"), run config (desabilitado, motivo), Run/Debug existentes, busca (entra na F3, botão presente e desabilitado com motivo) e Settings (desabilitado até F8, com motivo).
- [x] A status bar mostra breadcrumb solução > projeto > arquivo, `Ln/Col`, `UTF-8` e indentação, mantendo workspace, modo, SDK, avaliação, buffers sujos e keymap.
- [x] Encoding é declarado UTF-8 fixo (sem alegar detecção) e indentação vem das settings, não de análise do arquivo.
- [x] Todos os `data-testid` existentes são preservados e a suíte E2E inteira passa sem alteração de specs.
- [x] Faixas e divisores são operáveis por teclado, com nome acessível, `aria-pressed` e foco visível.
- [x] `bunx tsc --noEmit` e a suíte Playwright completa passam; commit local `feat(F1): ...`.
