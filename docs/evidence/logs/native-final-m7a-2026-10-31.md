# Native final — M7a bundle 810e70a8 (2026-10-31)

Bundle: `src-tauri/target/release/bundle/macos/OpenRidr.app`, binário SHA-256 `810e70a8f00ef53c1a88813efa7a4d3bf9cde14a95174d9ac7c583e82bcc3b46` (origem e cópia `/tmp/ponto/app3`). HEAD de código `f7212cd` (ticket 34). Processo PID 99916, janela 54044, `OPENRIDR_HOME=/tmp/ponto/home`, workspace em Trusted.

## Evidências deste SHA (native-50 a native-60)

| Item | Resultado observado | Arquivo |
|---|---|---|
| 8 Rename 1100×700, Problems aberto | Prévia docada: "Rename preview: 3 files · 6 edits · nothing has been applied", 3 linhas de arquivo, Apply to buffers / Cancel | native-50 |
| 8 Apply 1100 | "Rename applied to 3 files; 3 buffers are modified and unsaved. Nothing was written to disk" | native-51 |
| 9 Code actions 1100, Problems aberto | "4 actions from csharp-ls … nothing has been applied", as 4 ações e Close visíveis | native-52 |
| 9 Prévia de ação 1100 | "Code action preview: Introduce local … 1 file · 2 edits; nothing has been applied", Apply/Cancel visíveis | native-53 |
| 10 Find Usages 1100 | "Find Usages: 1 reference from csharp-ls; the list is read-only", Close | native-54 |
| 10 Find Usages 1440 + navegação | Lista docada, Close visível; clique na referência seleciona a linha e move o caret (Ln 4, Col 9) | native-55 |
| 20 Symbols `Program` 1440 | "No results" (sem "truncated by the host") | native-56 |
| Tab/foco 1440 | Após Tab com prévia aberta, foco vai para "Apply to buffers" (primeiro controle depois do editor, ordem visual) | native-57 |
| 8 Cancel inerte 1100 | "rename cancelled; no buffer or file changed" | native-59 |
| 8 Esc 1440 | Esc cancela a prévia ("rename cancelled…") | native-60 |
| 12 Build/Run após Apply | Build pede "Build with unsaved buffers?", build do conteúdo salvo → "Run saved build" → "Order A-104: R$ 42,00", "Process finished with exit code 0" | native-58 |

Cancel de code action e de rename verificados também neste SHA? Rename Cancel sim (native-59); code action Cancel: clique em Cancel na prévia, sem captura dedicada neste SHA (inércia observada no SHA `35038ee6`, r6/r8).

Disco da cópia `/tmp/ponto/OrderDesk` limpo após Apply/Cancel (`git status` vazio).

## Observações não classificadas

- A lista de code actions não usa o texto "Showing N of M"; o resumo é "N actions from csharp-ls …". A pergunta de texto do lead não se aplica a este build, e o resumo cumpre a função de contagem.
- Symbols `Program`: uma linha transitória `program` apareceu com "0 visible results" logo após digitar. Na reabertura ficou "No results". Não reproduzido de forma estável; registrado como observação.

## Lacunas (o que falta provar e por quê)

- Item 7 completion: Ctrl+Space e digitação não abriram popup nesta sessão (gatilho do helper; não confirmado se é o macOS). Não concluir defeito.
- Item 5 fechar aba suja por teclado: sem caminho confirmado (⌘W fecha o app, ⌘F4 sem efeito).
- Item 16: Rerun failed, cancelar, TRX expandido, MTP e Run/Debug selected não exercitados.
- Item 21: validação observada no SHA `35038ee6` (r10): working directory inexistente bloqueia Run ("does not exist or is not a directory"; "saved but cannot run until these fields are fixed"); `.env` vazio bloqueia Apply ("Remove the empty .env file row or fill its path"; "nothing was saved"). Diálogo travou depois (cliques e Esc sem efeito) e foi reiniciado. Não publicado (sem captura limpa). Faltam: Environment var com chave vazia, `.env` válido carregado e troca de Configuration/TFM.
- Item 2: Revoke com build/Run ativo não exercitado.
- M7b (campo inline): aberto, não implementado.
- Windows e macOS Intel não executados.
