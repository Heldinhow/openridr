# Commit e diff — mapa inicial contra Rider New UI Dark

Referências fornecidas pelo usuário, não copiadas ao repositório.

## Estado antes da implementação

- `src/git.tsx`: Commit tem seleção individual, contador, mensagem, amend funcional e diff dividido em diálogo; falta grupo tri-state, toolbar de ícones, nomes coloridos e caminhos na mesma linha. Diff é texto de patch alinhado, sem números, destaques intra-linha, navegação ou aba de editor.
- `src/WorkspaceView.tsx`: monta Commit na lateral; outro diff do editor é diálogo separado. `src/rider-chrome.css` só estiliza rótulos do split. CSS existente de Commit está em `src/styles.css`.
- `crates/host/src/git.rs` e dispatch em `crates/host/src/lib.rs`: status, diff, commit (inclui amend), operações remotas incluindo push; não há operação combinada Commit and Push, nem stash/rollback no Commit. Não inventar operações.
- Diff do host é patch textual limitado, não versões completas de arquivo; apresentar somente o que o host devolve e não inventar linhas. `@codemirror/merge` não está nas dependências diretas.
- Referência pede aba do editor, navegação de arquivos e diferenças, modo unificado/dividido, faixas conectadas e destaque intra-linha; estes são gaps separados do painel.

## Entrega parcial

- Painel Commit: grupo recolhível com seleção tri-state, ícones por arquivo, nome e diretório na mesma linha, cores por status, toolbar, amend funcional já existente, mensagem maior e botão combinado desabilitado com motivo. Rollback desabilitado com motivo; push separado já existe no menu de branch. Sem alteração no host.
- Não implementado: diff como aba do editor, CodeMirror, numeração, conectores, intra-linha, navegação e alternância; continua diálogo pré-existente com patch dividido. Abertura por clique único foi preservada; duplo clique/Enter também acionam o botão, mas ainda abrem diálogo.
- Não há nova operação Git nem dependência; arquivos retornados pelo host não são filtrados.

## Verificação

- `bun run check`: passou; `bun test tests/unit`: 33 passaram.
- E2E Git (portas 1560/4457): 8 passaram, 1 falhou por seletor ambíguo no teste novo; seletor corrigido e teste afetado passou isoladamente. Não houve nova bateria completa. O teste existente foi ampliado para grupo tri-state e recolhimento.
- Superfície B em host e fixture temporários: capturas 1470×880 em `rider-git-commit-diff/` antes/depois. A captura anterior do diff foi feita durante carregamento e não comprova seu conteúdo; a posterior espera o patch. A captura do Commit mostra a lista e os controles. Nenhuma modificação na fixture versionada.
- App desktop não verificado, conforme pedido. macOS Intel/Windows não verificados.
