# 03: CRLF e UTF-16 preservados

**What to build:** Salvar um arquivo CRLF mantém CRLF; abrir e salvar um arquivo UTF-16 com BOM mantém encoding e BOM. Arquivo UTF-8/LF continua igual. O conflito de disco compara o texto normalizado, não os bytes crus.

**Blocked by:** None (can start immediately).

**Status:** implementação concluída — prova no app Tauri real feita em 2026-11-04 (`docs/evidence/fix-alpha5/VERIFICATION.md`).

- [x] Arquivo CRLF: abrir, editar, salvar → todas as linhas continuam CRLF.
- [x] Arquivo LF: abrir, editar, salvar → continua LF.
- [x] Arquivo misto preserva o estilo predominante (primeira ocorrência de CRLF define CRLF).
- [x] UTF-16LE e UTF-16BE com BOM abrem no editor sem erro de UTF-8.
- [x] Salvar um UTF-16 grava o mesmo encoding e o BOM.
- [x] UTF-8 com BOM preserva o BOM; UTF-8 sem BOM continua sem BOM.
- [x] O hash de conflito é estável entre leitura e escrita (texto normalizado dos dois lados).
- [x] Bytes inválidos/encoding desconhecido continuam erro claro, sem corromper o arquivo.
- [x] Testes de host para round-trip e conflito; e2e para abrir/salvar CRLF e UTF-16.
- [x] Prova no app Tauri real: abrir/salvar CRLF e UTF-16 no bundle (`verify-openridr`).
