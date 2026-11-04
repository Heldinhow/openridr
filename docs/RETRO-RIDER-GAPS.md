# Retro: gaps OpenRidr × Rider

Data: 04/11/2026. Branch `docs/rider-gaps`. Entregáveis: `RIDER-BEHAVIORS.md`, `GAPS-RIDER-RIDR.md` e este retro. Nenhuma correção de código foi feita.

## O que foi verificado

- **OpenRidr, na superfície web isolada** (`verify-openridr/web.sh`, UI 1520 e host 4417, cópia do OrderDesk):
  - Welcome, confiança, árvore da solução.
  - Menus de contexto de solução, projeto, pasta e arquivo.
  - Tests: descoberta, Run all, resultado por teste, detalhe de falha, Debug selected desabilitado.
  - Search Everywhere com filtro e prévia.
  - Find do editor (painel CodeMirror).
  - Breakpoint, Debug até pausar, locals (expandem), Step Over, Stop.
  - Diálogo de configuração de execução.
  - Find Usages sem análise instalada.
  - Build com sucesso.
  - Settings > Keymap.
- **Keymap base do pacote do Rider**, lido passivamente (atalhos apenas, nada copiado).
- **Descritores de plugin do Rider**: contagem de ações e grupos.
- **Comparativo anterior** (29/10) reconferido em dois pontos. As variáveis expansíveis e o rodapé do diálogo de configuração já estão resolvidos.

## O que NÃO foi verificado

- **Rider, na UI:** nenhuma. O `agent-desktop` está sem Acessibilidade e sem Gravação de Tela. Todo comportamento de UI do Rider que está no doc vem de 29/10 ou é marcado "não verificado".
- **Janela nativa do OpenRidr (Tauri):** não foi aberta. Os testes foram na superfície web, que usa a mesma UI mas não a janela nativa.
- **Análise (csharp-ls):** "Install analysis" não foi acionado, porque baixa software de rede sem pedido explícito. Por isso Find Usages, Rename e code actions não foram testados de verdade.
- **Build com erro:** não provocado, então os links `arquivo:linha` no Build output continuam sem verificação (G20).
- **Modificadores do keymap base no macOS:** a correspondência Ctrl→⌘ e Alt→⌥ é presumida (G02, G04, G05).

## O que o REA deu de evidência

- **Pouco.** Abri o executável `Rider` (Mach-O arm64) e obtive o inventário do binário.
- `search_strings` falhou: nenhum provedor configurado. Ghidra precisa de `GHIDRA_INSTALL_DIR` e Java; Hopper não está instalado; IDA e Kuna não foram configurados.
- O REA não é o caminho para isto. Os textos de UI e os keymaps estão em jars e recursos, não no launcher. O keymap e os descritores foram lidos por `unzip`, o que é leitura passiva de artefato, não análise REA.
- Para dar valor ao REA, seria preciso configurar um provedor (Ghidra ou Hopper) e abrir o `rider-frontend.jar` ou a biblioteca `intellij.platform.ide.impl.jar` via `inspect_artifact`. Não foi feito.

## Bloqueios e próximos passos

1. **Permissão do macOS (do usuário):** em Ajustes do Sistema > Privacidade e Segurança, liberar Acessibilidade e Gravação de Tela para o app que roda o terminal. Depois, rodar `agent-desktop status` e repetir a observação visual do Rider nos itens de "Verificações pendentes".
2. **Análise do OpenRidr:** decidir se o "Install analysis" entra no teste de Find Usages (G01). Exige aprovação, porque baixa ferramenta.
3. **Keymap:** decidir G02 e G05 e registrar em `docs/decisions.md` antes de mudar atalhos.

## Erros e ajustes deste retro

- Primeiro ⌘F foi enviado sem foco no editor e não abriu o painel. Depois de clicar no editor, abriu. Isso virou G06.
- O cursor do teste de Find Usages ficou na linha 5 em vez da linha 3. O aviso de "análise necessária" aparece independentemente, então a conclusão não muda, mas o teste não foi limpo.
- `docs/RIDR-VS-RIDER.md` ficou desatualizado em dois pontos (variáveis e rodapé). Não foi editado, porque está fora do escopo. Sugestão: atualizar em um commit próprio.
- Os screenshots ficaram em `.scratch/rider-gaps/` e não foram versionados. `.playwright-mcp/` também é ignorado pelo git.
