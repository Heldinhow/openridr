# 03: CRLF e UTF-16 preservados

**What to build:** Salvar um arquivo CRLF mantém CRLF; abrir e salvar um arquivo UTF-16 com BOM mantém encoding e BOM. Arquivo UTF-8/LF continua igual. O conflito de disco compara o texto normalizado, não os bytes crus.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Arquivo CRLF: abrir, editar, salvar → todas as linhas continuam CRLF.
- [ ] Arquivo LF: abrir, editar, salvar → continua LF.
- [ ] Arquivo misto preserva o estilo predominante (primeira ocorrência de CRLF define CRLF).
- [ ] UTF-16LE e UTF-16BE com BOM abrem no editor sem erro de UTF-8.
- [ ] Salvar um UTF-16 grava o mesmo encoding e o BOM.
- [ ] UTF-8 com BOM preserva o BOM; UTF-8 sem BOM continua sem BOM.
- [ ] O hash de conflito é estável entre leitura e escrita (texto normalizado dos dois lados).
- [ ] Bytes inválidos/encoding desconhecido continuam erro claro, sem corromper o arquivo.
- [ ] Testes de host para round-trip e conflito; e2e para abrir/salvar CRLF e UTF-16.
