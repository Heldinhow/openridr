# Retro — QA top 4 (2026-11-02)

Escopo: achados 1–4 de `QA-BUGHUNT-2026-11-02.md`; spec em `SPEC-QA-TOP4.md`, decisões em `decisions.md`.

## O que funcionou

- Teste vermelho primeiro para cada achado (qa1–qa4): os três que não dependem de rede falharam pelo motivo certo antes do código.
- Reuso: `lspStartBlocker` saiu do `LspPanel` e virou a única regra de "pode iniciar"; `withLocalEdits` e `isBuildObsolete` já existentes decidem quando uma marca no editor deixa de valer.
- Sem dependência nova: o `@codemirror/lint` não estava instalado; `Decoration` + `gutter` do `@codemirror/view` bastaram.

## O que surpreendeu

- Um teste existente (D-12 do `csharp-ls.spec.ts`) afirmava que "Modified" depois do undo era legítimo. Era o bug do achado 3 virado contrato; foi atualizado, com a decisão registrada.
- O auto-start muda o ponto de partida dos specs do csharp-ls (a análise já pode estar rodando quando o spec chega no botão do painel).

## Review do diff contra o spec

- Achado 1: o spec dizia "uma vez por workspace"; o código ficou "uma vez por fronteira" (`workspace:contextId:restore.revision`) porque o host para a análise sozinho ao iniciar restore ou mudar input do projeto — sem isso o segundo restore deixava "Stopped" para sempre. Falha do auto-start é silenciosa (o host revalida os assets e pode recusar). Spec atualizado em `decisions.md`.
- Achado 1, limite: um launcher já no disco de uma sessão anterior continua `absent` até o plano ser confirmado (o host só executa o launcher após a confirmação). A barra mostra "Install analysis" e o plano reutiliza sem baixar. Mudar isso é decisão de confiança do host, não feita aqui.
- Achado 2: o item de status do achado 1 sobrepunha o contador de erros (visto em screenshot, corrigido em commit próprio). Ninguém havia notado só com asserts de DOM.
- Achados 3 e 4 batem com o spec; a build com avaliação falha continua desabilitada (fora do escopo).

## Dívida / não feito

- Build continua desabilitado com avaliação falha ("Evaluate the project model before building"): fora do escopo, só o item de árvore/Problems foi tratado.
- O botão de início ficou na barra de status global, não dentro da faixa `editor-lsp-status` do editor.
- Marcas LSP somem enquanto o buffer está "awaiting" (a cada edição) e voltam na próxima publicação.
- App Tauri real e Windows/macOS Intel: não verificados (só ponte HTTP + Chrome).
