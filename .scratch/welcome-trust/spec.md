# Spec: Welcome redesenhada + Trust decision única por projeto

Status: ready-for-agent
Decisões: `docs/decisions.md` (2026-11-01), termos em `GLOSSARY.md` (Recent project, Trust decision, Trust prompt, Restricted mode).

## Problem Statement

A tela inicial do OpenRidr é uma rolagem longa de formulário, SDKs e processos, longe da welcome do Rider. Escolher um projeto recente não mostra quando foi aberto, não diz se é confiável e não tem estados claros de vazio/erro. Além disso, o indicador "Trusted/Restricted mode" ocupa a barra de cima e um banner de aviso reaparece toda vez que um projeto restrito é aberto, porque só a decisão "confiar" é lembrada.

## Solution

Uma welcome de qualidade de produto: lista de *Recent projects* com nome, caminho encurtado, último acesso e selo de trust, busca, ações e atalhos, estados vazio e de erro; Toolchain numa segunda vista. Ao abrir um projeto sem *Trust decision*, um *Trust prompt* modal pergunta uma única vez (Trust project / Open in Restricted mode / Cancel). A decisão é gravada por projeto e nunca perguntada de novo. O indicador sai da barra de cima; a informação continua visível e mutável na barra de status e na lista da welcome.

## User Stories

1. Como dev, quero ver meus projetos recentes com nome, caminho e último acesso, para reconhecer qual abrir.
2. Como dev, quero filtrar a lista digitando, e ver quantos resultados restam, para achar um projeto antigo rápido.
3. Como dev, quero abrir um recente com clique ou Enter, para entrar sem fricção.
4. Como dev, quero navegar a lista com ↑/↓, `/` para a busca e ⌘O para abrir uma pasta, para usar só o teclado.
5. Como dev, quero um estado vazio claro na primeira execução, com a ação Open em destaque, para saber o que fazer.
6. Como dev, quero que um projeto cujo caminho sumiu ou falhou ao abrir mostre o erro na própria linha, para entender o que houve sem perder o contexto.
7. Como dev, quero remover um projeto da lista sem tocar em arquivos nem na minha decisão de trust, para limpar o histórico.
8. Como dev, quero ser perguntado sobre confiança apenas na primeira vez que abro um projeto, para não ser interrompido a cada abertura.
9. Como dev, quero que o prompt ofereça Trust project, Open in Restricted mode e Cancel, com foco inicial em Trust project, Esc = Cancel, Enter no botão focado e Tab circulando, para decidir com teclado e entender as consequências.
10. Como dev, quero que Cancel volte à welcome sem abrir nada, para desistir sem efeito.
11. Como dev, quero que minha decisão (inclusive restrito) seja lembrada entre sessões, para nunca ser reperguntado.
12. Como dev, quero que a barra de cima não mostre mais "Trusted mode", para ela ficar só com ações do dia a dia.
13. Como dev, quero ver o modo na barra de status e mudá-lo por um popover pequeno (Trust/Restrict), para revisar a decisão sem sair do projeto.
14. Como dev, quero ver o selo de trust de cada recente e mudar a decisão pelo menu "⋯" sem abrir o projeto, para revogar confiança de um projeto antigo.
15. Como dev que já usava o app, quero que meus projetos confiados continuem confiados e que os demais recebam a pergunta uma vez, para não perder nada nem ter decisão presumida.
16. Como dev, quero que arquivos de estado antigos ou corrompidos não travem o app nem sejam sobrescritos por uma leitura, para não perder dados.
17. Como dev, quero ver SDKs/adaptador na vista Toolchain, e um aviso discreto em Projects se o host de análise faltar, para a welcome ficar limpa sem esconder problemas.
18. Como dev que usa o formulário de caminho (navegador/E2E), quero o mesmo Trust prompt, para um só fluxo.
19. Como dev, quero foco visível e leitor de tela corretos no modal e na lista, para a welcome ser acessível.

## Implementation Decisions

- **Trust store (host):** `trusted.json` vira mapa caminho canônico → `trusted|restricted`. Leitura tolerante ao formato lista (= `trusted`) e a arquivo ausente; arquivo ilegível não é sobrescrito por leitura (gravar só após leitura bem-sucedida ou arquivo ausente). Gravação atômica existente. `OPENRIDR_TRUST_STORE=off` mantido.
- **Métodos de host novos:** `trust.get {path}` → decisão ou nulo; `trust.set {path, decision}` persiste a decisão de qualquer caminho; quando é o workspace aberto, aplica/revoga com as garantias existentes de grant/revoke (processos, contexto, selagem). `trust.grant`/`trust.revoke` continuam e passam a gravar `trusted`/`restricted`. `trust.revoke` grava `restricted` (não esquece).
- **Abertura:** `workspace.open` aplica trust automaticamente só quando a decisão gravada é `trusted`; devolve no snapshot a decisão gravada (`trust.decision: trusted|restricted|null`) para a UI saber se deve perguntar. A UI pergunta quando `null`.
- **Recent store (host):** `recent.json` vira `[{path, openedAt}]`; leitura aceita o formato lista; `recent.list` devolve `{projects:[{path, openedAt?, decision}]}` mantendo ocultas as entradas cujo arquivo não existe; novo `recent.remove {path}`. Compatibilidade: o campo antigo de caminhos continua útil à toolbar via lista de caminhos derivada na UI.
- **UI — Trust prompt:** componente modal único (dialog acessível, foco preso, foco inicial em Trust project, Esc = Cancel). Reutiliza `TrustControls` para a explicação do que a confiança autoriza. Usado pela lista, pelo seletor e pelo formulário de caminho; o card "Review workspace trust" é removido.
- **UI — Status e toolbar:** `mode-badge`, `trust-banner`, `trust-prompt*` saem da toolbar; `status-mode` na barra de status vira botão com popover (Trust/Restrict), com Esc e retorno de foco.
- **UI — Welcome:** Projects como vista principal, navegação lateral Projects/Toolchain, lista com nome, caminho encurtado com `~`, último acesso relativo, selo de trust, menu "⋯" (Trust/Restrict, Remover da lista), busca com contagem, estado vazio, erro por linha, atalhos. Só tema escuro. Direção visual segue o Rider.app (capturar referência) e a skill impeccable (`craft-floor.md` lido antes de editar UI); tipografia de piso 12px, espaçamento consistente, tokens existentes; sem dependência nova.
- **Fora da welcome:** o outline de texto pós-abertura sai (entra-se direto); Toolchain e painel de processos mudam para a segunda vista sem perder `data-testid` existentes.

## Testing Decisions

- Um seam principal: **Playwright E2E contra host real e fixture real** (prior art: `trust-sdk.spec.ts`, `gates.spec.ts`, `workspace.spec.ts`), mais testes unitários de `cargo test` nos dois stores (prior art: testes de `settings.rs`).
- Bom teste = comportamento observável: prompt aparece uma vez, não reaparece após reabrir (estado em app home isolado via `OPENRIDR_HOME`), barra de cima sem o indicador, info no novo lugar, migração dos formatos antigos, Esc/Enter/Tab do modal.
- Specs afetados migram do `mode-badge` para o prompt/`status-mode`: `gates`, `trust-sdk`, `workspace`, `ui-layout`, `zy-restore-interrupt`, `restore`, `code`, `csharp-ls`, `zz-acceptance`, mais os que dependem da welcome (`ui-layout`).
- Verificação real final: Chrome visível (não headless) cobrindo welcome e fluxo de trust; capturas da welcome e do modal em `docs/evidence`.
- Checks: `bun run check`, `cargo test`, specs E2E afetados.

## Out of Scope

New project (`dotnet new`) e Clone (`git clone`); tema claro; fonte Inter embutida; reformatar o repo; push/PR/merge.

## Further Notes

`cargo fmt`/`clippy` não passam hoje: formatar só o tocado. Sem `--no-verify`. Commits convencionais, um por ticket, `git commit --only -- <paths>`. Registrar em `docs/implementation-notes.md` o que não foi verificado.
