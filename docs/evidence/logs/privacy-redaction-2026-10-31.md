# Privacy redaction — native-*.png (2026-10-31)

Revisão independente achou prompt zsh com usuário/host e caminhos pessoais em capturas nativas publicadas.

- Redigidos (caixas pretas sobre palavras detectadas por OCR: helder, MacBook, /Users, /private/var, Air-de): native-00, 07, 08, 09, 17, 18, 19, 20, 47.
- Verificação por OCR após redação: 0 ocorrências nos nove arquivos.
- native-26 (OCR limpo, sem alteração).

Histórico local: commits anteriores (ex.: 1e7e123, 6f58add, c37c701, 3092d16) contêm a versão original de native-19/20 com host/usuário no blob. NÃO publicar o branch sem filtrar esses blobs antes (ex.: git filter-repo ou reescrita controlada, decisão do mantenedor).

Fora de escopo deste worker (não gerados por ele): 16-*, 17-*, 18-*, d05, d09, f5, ui-trust-screen e 09/10/m7 do m7-impl. Ver lista na mensagem ao lead.
