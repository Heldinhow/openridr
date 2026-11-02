# SPEC — médios de editor, estado e primeiro uso (2026-11-02)

Escopo: corrigir somente os oito achados médios pedidos, sem tocar nos baixos nem em itens fora da lista.

| ID | Achado | Contrato de aceite |
|---|---|---|
| d9 | Reabrir o workspace perde abas, cursor e breakpoints | O store por solução guarda `tabs[]` (caminho + linha/coluna), `activePath` e `breakpoints` por arquivo; ao reabrir, as abas são lidas do disco, a ativa/cursor são restaurados e os breakpoints voltam (só com Trusted). |
| d13 | Duplo clique abre duas abas idênticas | Duas aberturas concorrentes do mesmo caminho resultam em uma única aba. |
| b14 | Buffer limpo não recarrega com mudança no disco | Buffer sem edições é substituído pelo texto do disco em `checkDisk`; buffer sujo continua recebendo `conflict` e nunca é sobrescrito. |
| b15 | Janela pequena esconde o editor | Altura do painel inferior é limitada para deixar ao menos 180 px de editor; largura do painel esquerdo cede em viewports estreitas; sem overflow horizontal. |
| d19 | Problems com altura e contagens contraditórias | Altura padrão do painel inferior sobe para 320 px; contagem do cabeçalho soma diagnósticos atuais + build atual + descoberta atual + projetos com avaliação falha, igual à barra de status e ao rótulo da aba; o aviso de "nenhuma sessão de análise" não aparece quando há problemas de build/testes/avaliação. |
| b11 | NuGet mostra `Current: unknown` | O parser aceita `requestedVersion`/`resolvedVersion` (além de `requested`/`resolved`); a coluna Current mostra a versão resolvida real. |
| b13 | MissingSdk é beco sem saída | O painel de avaliação diz qual SDK instalar, que o host não instala nem substitui, e oferece "Redetect SDKs", link oficial de downloads e "Open global.json". Nada é instalado automaticamente. |
| b10 | Terminal não recebe foco ao abrir | Ao montar a aba e quando o shell fica running, o foco vai para o textarea do xterm; digitar imediatamente funciona sem clicar no terminal. |

## Testes

- E2E Playwright: `tests/e2e/medium-editor-state.spec.ts` (d13, b14, b15, d19, b13, b10) e `tests/e2e/medium-editor-session.spec.ts` (d9, host isolado com `OPENRIDR_TRUST_STORE=on`).
- O host compartilhado do Playwright roda com `OPENRIDR_SESSION_STORE=off`; sem isso a sessão de um spec restaura abas/cursores no seguinte. O app e o spec de sessão mantêm o store ligado.
- Rust: `run_configs` (roundtrip/preservação da sessão) e `nuget::parse_listing` (campos `*Version`).
- Regressão: suíte E2E completa e `cargo test --workspace`; falhas pré-existentes esperadas: `find-usages`, 2 de `run-configs`, 2 de `tests.spec` e `file_access_is_confined_to_the_workspace`.

## Fora de escopo

Achados baixos, restauração de conteúdo não salvo, `Recent Files` persistido, regex/máscara de busca, debug de teste, instalação automática de SDK ou de ferramentas. Sem push, PR ou release.
