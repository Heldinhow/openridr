# Review — welcome-trust (2d242ab..HEAD, branch feat/welcome-trust)

Revisão estática de código, spec e standards. Nada foi executado nem editado. Eixos separados: **A Standards**, **B Spec**.
Veredito: **aprovar com ressalvas** — 0 blockers, 3 majors a corrigir antes do merge.

## Majors

### 1. [major · B/segurança] `trust.set` tem TOCTOU: pode confiar em outro workspace — `crates/host/src/lib.rs:1030-1048`
`set_trust` grava a decisão, solta o lock depois de calcular `is_open` e chama `grant_trust`/`revoke_trust`, que pegam o lock de novo e agem sobre **o workspace aberto naquele instante**. Se um `workspace.open` entrar entre os dois locks, `grant_trust({remember:true})` concede e **persiste** `trusted` para o workspace novo, que o usuário nunca decidiu. O Tauri e a ponte HTTP chamam o host em paralelo, então a janela existe.
Recomendação: decidir e aplicar sob um único lock. Extraia `grant_locked(&mut inner, remember)` e `revoke_locked(&mut inner)` e chame-os de `set_trust` com o mesmo guard. Ou, na aplicação, reconfira `workspace.path == path` dentro do lock.

### 2. [major · B] Cancel não é "sem efeito" — o projeto entra em Recentes — `crates/host/src/lib.rs:623-626`, `src/App.tsx:67-70`
`recent::record` roda dentro de `workspace.open`, antes da decisão. Cancel/Esc chama `workspace.close`, mas o projeto já está em Recentes com selo "Not decided". Se já estava, ganha `openedAt` novo e sobe ao topo. Isso contraria a US 10 ("desistir sem efeito") e o ticket 03 ("Cancel volta à welcome sem abrir"). A observação aparece só em `docs/evidence/welcome-trust/VERIFICATION.md:25`. Não há decisão em `docs/decisions.md`, e o rótulo "fica para decisão do dono" não substitui uma.
Recomendação: registrar o recente só depois da decisão (Trust/Restricted ou abertura de projeto já decidido), ou fazer Cancel restaurar a entrada anterior. Se a escolha for manter, registre em `decisions.md` e acrescente um teste que afirme o comportamento escolhido.

### 3. [major · B] Cancel/Esc destrói o workspace aberto e corre contra uma decisão em voo — `src/App.tsx:63-66`, `src/TrustPrompt.tsx:42-44`
- **Perda do workspace anterior.** `openRecent` (`src/App.tsx:86`, usado pela toolbar do IDE) faz `workspace.open`, que já substitui o workspace atual no host. Se o projeto novo for indeciso e o usuário der Cancel, `workspace.close` deixa o host **sem workspace nenhum** e o usuário cai na welcome. Ele pediu "desistir" e perdeu o projeto em que estava, com os processos marcados `awaiting`.
- **Esc durante `busy`.** Os botões ficam `disabled` durante `decide`, mas o Esc em `onKeyDown` não verifica `busy`. Esc depois de o `trust.set` ter sido enviado fecha o workspace. A resposta chega depois e `decided()` chama `handleSnapshot` e `setStage("workspace")` com o snapshot de um workspace já fechado, e a decisão fica gravada apesar do Cancel.
- **`workspace.close` sem tratamento de erro.** `cancelPrompt` (`src/App.tsx:63-66`) faz `setPrompt(null)` antes do `await` e não tem `try/catch`. Se o close falhar, a rejeição fica sem tratamento, o modal some e o host segue com o workspace aberto.
Recomendação: ignorar Esc e Cancel enquanto `busy`; envolver `cancelPrompt` em try/catch e só limpar o prompt após o close. Para a perda do workspace anterior, peça a decisão antes de o host trocar o workspace, ou reabra o anterior no Cancel. Cobrir com um spec.

## Minors

### 4. [minor · A] Piso e contrato de `data-testid`: ids removidos sem registro — `src/SetupScreen.tsx` (removidos), `docs/decisions.md:419`
A exceção de 2026-11-01 lista `mode-badge`, `trust-banner`, `trust-prompt`, `trust-prompt-open` e `trust-prompt-dismiss`. Também sumiram `open-text-only` e `workspace-path` (removidos do `SetupScreen`; só sobrou um comentário em `tests/e2e/gates.spec.ts:444`). A spec manda o outline sair, mas a lista da exceção não os cita.
Recomendação: acrescentar os dois à exceção em `decisions.md`.

### 5. [minor · A] `messageOf` repetido em vez de reutilizado — `src/SetupScreen.tsx:20`, `src/welcome/ProjectsView.tsx:23`, `src/welcome/ToolchainView.tsx:13`, `src/TrustPrompt.tsx:28`
AGENTS.md pede "uma função de erro por fronteira (`messageOf`...)". `src/host/client.ts:1109` já exporta `messageOf`. O diff acrescentou mais cópias locais e um mapeamento inline (`TrustPrompt`), e `TrustControls.tsx:14` continua com a sua. Recomendação: importar de `host/client`.

### 6. [minor · A] Arquivos indevidos commitados — `.scratch/welcome-trust/verify-visible.mjs`, `docs/evidence/welcome-trust/verify-visible.log`
O `.mjs` é script de verificação descartável em `.scratch`. O `.log` sozinho é evidência aceitável, mas sem o script não é reproduzível. As capturas de logs e screenshots regravadas pelos specs foram revertidas: nenhum arquivo antigo de `docs/evidence/{logs,screenshots}` aparece modificado. Recomendação: tirar o `.mjs` do histórico da branch (ou do índice) se não for para versionar; manter log e PNGs só se forem evidência.

### 7. [minor · B] Migração de `recent.json` pode sobrescrever dado ilegível — `crates/host/src/recent.rs:23-35, 62-67`
A US 16 pede que arquivos antigos ou corrompidos não travem o app nem sejam sobrescritos. `trusted.rs` trata isso (`read_all` devolve `None`, `set` não grava). `recent::read_all` devolve `Vec::new()` para qualquer erro, então o próximo `record()` ou `remove()` sobrescreve um `recent.json` ilegível com uma lista nova. Itens de formato desconhecido também são descartados na reescrita. Não há teste para o caso. Recomendação: o mesmo padrão `Option` de `trusted.rs` e um teste "arquivo ilegível não é sobrescrito".

### 8. [minor · B/segurança] `trust.set` aceita qualquer caminho absoluto, inexistente ou não — `crates/host/src/lib.rs:5883-5892`
`trust_key` canonicaliza se o caminho existe e, senão, usa o texto cru. Qualquer cliente da ponte pode gravar entradas arbitrárias e sem limite em `trusted.json`. Symlinks são resolvidos na gravação e o `workspace.open` compara com o caminho canônico, então a falha é segura: um symlink criado depois não herda a decisão. Recomendação: exigir que o caminho exista (ou que seja um `.sln`/`.csproj`/diretório) e rejeitar o resto com `not_found`. Deixa de acumular lixo no arquivo.

### 9. [minor · B] `trust.set` com store ilegível ou desligado responde sucesso sem gravar — `crates/host/src/trusted.rs:55-66`
`set` é "best effort" e silencioso. Com `trusted.json` corrompido, o prompt devolve o snapshot normalmente, a UI entra no IDE e a pergunta volta na próxima abertura, sem aviso. O comportamento é coerente com a spec ("não sobrescrito"), mas o usuário não sabe. Recomendação: devolver erro (ou um campo `persisted:false`) para a UI mostrar uma nota.

### 10. [minor · A11y] Modal sem foco realmente preso — `src/TrustPrompt.tsx:34-44`, `src/App.tsx:108`
O Tab circula só se o foco já estiver dentro do diálogo. Após um clique no backdrop, o foco vai para o `body` e o Tab chega ao fundo, porque a welcome não fica `inert`/`aria-hidden`. Leitores de tela também alcançam o fundo. A US 19 pede "foco visível e leitor de tela corretos". Recomendação: `inert` na welcome enquanto houver `prompt`, ou `autoFocus` de volta no Tab fora do diálogo.

### 11. [minor · A11y] Popover de status e atalhos da welcome — `src/shell/StatusMode.tsx:43-72`, `src/welcome/ProjectsView.tsx:35-47`
- O popover de `status-mode` tem `role=dialog` e `aria-haspopup=dialog`, mas ao abrir o foco não vai para dentro: o teclado precisa de Tab para chegar ao checkbox, e o Esc só funciona com o foco no botão ou no popover.
- Os atalhos `/` e ⌘O do `ProjectsView` ficam montados com a vista Toolchain ativa (`SetupScreen` só usa `hidden`), então `/` foca uma busca invisível. Recomendação: focar o primeiro controle ao abrir, e desligar os atalhos quando `view !== "projects"`.

## Nits

### 12. [nit · perf] Leitura do arquivo por snapshot e por linha — `crates/host/src/lib.rs:824-827`, `crates/host/src/recent.rs:46-60`
`trusted::get` lê e parseia `trusted.json` a cada `snapshot_value` e a cada entrada de `recent.list`, até 30 vezes por chamada. Para o tamanho atual não importa. Recomendação (se crescer): ler o mapa uma vez por `list`.

### 13. [nit · A] Duplicação de `read_all` e escrita com `Option<u64>` — `crates/host/src/recent.rs:15-43`
`write_all` reconstrói JSON com um `match` para cada entrada. Dá para guardar `Value`/struct com `serde` e remover o `match`. Estilo, sem impacto.

### 14. [nit · B] Corrida entre processos no read-modify-write de `trusted.json`/`recent.json` — `crates/host/src/trusted.rs:55-66`
Duas instâncias (app + ponte dev) podem perder uma gravação. A escrita atômica evita arquivo corrompido, não perda de atualização. Aceitável hoje; registrar como limitação em `implementation-notes.md`.

## Verificado e sem achado
- **Dependências:** nenhuma nova (nenhuma mudança em `package.json`/`Cargo.toml` no diff).
- **Comentários:** em inglês nos arquivos novos.
- **Um componente por arquivo:** respeitado em `src/welcome/*`, `TrustPrompt`, `StatusMode`.
- **Piso de 12px:** nenhum `font-size` abaixo de 12px no CSS do diff.
- **Commits:** convencionais, um por ticket; árvore limpa.
- **Migração de `trusted.json`:** lista antiga vira `trusted`, preservada e reescrita como mapa só na próxima gravação; arquivo ilegível não é sobrescrito (`unreadable_file_is_not_overwritten`).
- **`trust.revoke`:** grava `restricted`, não esquece.
- **`trust.set` no workspace aberto:** passa pelos mesmos caminhos de `trust.grant`/`trust.revoke`, com as garantias de revogação e `context_seq` (ressalva: achado 1).
- **UI x host:** a UI só chama `dispatch`, sem lógica de host duplicada.

## Cobertura por user story
US 1–3, 5–7, 11–14, 17–18 atendidas. US 4: ↑/↓, `/` e ⌘O presentes (ressalva do achado 11). US 8–9: atendidas. **US 10: parcial** (achados 2 e 3). **US 15–16: parcial** (achado 7 para `recent.json`; `trusted.json` ok). **US 19: parcial** (achados 10 e 11).

## Resolução
| # | Status |
|---|---|
| 1 | Corrigido: `set_trust` decide e aplica sob um lock (`grant_locked`/`revoke_locked`). |
| 2 | Corrigido: `workspace.open` só registra Recentes com decisão gravada; sem decisão registra em trust.set/grant/revoke. Decisão "Cancel não deixa rastro" em `decisions.md`; testes host + E2E. |
| 3 | Corrigido: Esc/Cancel ignorados com `busy`; `cancelPrompt` com try/catch e erro no modal; Cancel vindo do IDE reabre o workspace anterior (E2E). |
| 4 | Corrigido: `open-text-only` e `workspace-path` na exceção de `decisions.md`. |
| 5 | Corrigido: `messageOf` de `host/client` nos 5 lugares. |
| 6 | Corrigido: `.mjs` removido (`git rm`); referências apontam para o log. O `.log` fica no histórico como evidência. |
| 7 | Corrigido: `recent.json` ilegível não é sobrescrito + teste. |
| 8 | Corrigido: `trust.set` exige caminho absoluto existente (`not_found`) + teste. `trust.get`/`recent.remove` seguem lenientes. |
| 9 | Corrigido: falha de gravação vira erro (`trusted_unreadable`/`trusted_failed`) + teste. |
| 10 | Corrigido: fundo `inert` com o prompt aberto; backdrop não rouba foco (E2E). |
| 11 | Corrigido: foco entra no popover; `/` e ⌘O só na vista Projects. |
| 12 | Aceito: sem impacto no tamanho atual; dívida registrada. |
| 13 | Corrigido: `match` de `write_all` removido (`openedAt` nulo). Efeito: entradas legadas passam a gravar `"openedAt": null`. |
| 14 | Aceito: limitação registrada em `implementation-notes.md`. |
