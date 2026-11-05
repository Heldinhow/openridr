# Gaps iniciais — Tests, NuGet e setup de ferramentas

Levantamento estático em 2026-11-05, antes da implementação. As referências visuais fornecidas ficam fora do repositório. Este documento não afirma verificação visual nem execução no app.

## Tests

- Já existem descoberta por projeto/TFM, execução de todos ou selecionados, cancelamento, repetição de falhas, filtro textual, árvore agrupada, contagens por resultado, duração e logs (`src/TestsPanel.tsx`, `src/testTree.ts`).
- Falta aproximar a composição visual da referência: abas e sessão, barra vertical de ações, contadores por ícone, agregação de estado nos grupos e detalhe/saída do nó selecionado à direita. Preservar os `data-testid` existentes.
- Debug de testes está explicitamente indisponível: o host não oferece attach ao testhost. Continuous Testing e cobertura exigem capacidades próprias; não apresentá-los como funcionais sem implementá-las. Exportação e ações de expansão global precisam ser avaliadas durante a especificação.
- A árvore atual parte dos nomes totalmente qualificados descobertos; não assumir que a descoberta contém hierarquia real de pastas ou informações de origem para todos os testes.

## NuGet

- Já existem seleção de projeto, consulta de pacotes instalados e desatualizados por TFM, contagem de desatualizados, listagem de fontes, atualização individual mediante plano confirmado, logs e oferta separada de restore (`src/NuGetPanel.tsx`, `crates/host/src/nuget.rs`).
- Faltam composição e controles da referência: abas, busca, escopo Solution, seletor de fonte, prerelease, Installed/Discover, seções e seleção de updates em lote, além da apresentação de implícitos. Confirmar os dados e limites reais antes de prometer cada operação.
- O contrato atual de `nuget.list`, `nuget.planUpdate` e `nuget.applyUpdate` é por projeto e por pacote; o host não expõe uma operação de lote nem um catálogo Discover. O modelo atual de pacote não distingue implícitos/transitivos. Central Package Management é detectado, mas atualização é recusada; não contornar essa proteção.
- Updates atuais usam `dotnet add ... --no-restore`, com restore explícito posterior; uma operação em lote precisa tratar falhas parciais e manter os gates de segurança.

## Setup

- O fluxo atual tem plano e confirmação para instalação de csharp-ls, início manual da análise e flag experimental de sessão para o debug adapter (`src/ToolchainPanels.tsx`, `crates/host/src/lib.rs`). O host exige trust para `tools.install`.
- netcoredbg possui caminho de bundle e metadados de versão/checksum, além do fallback instalado; csharp-ls dispõe de fluxo de instalação e progresso. A disponibilidade real desses caminhos e a persistência/verificação devem ser comprovadas no app, não presumidas.
- Falta um gate de setup obrigatório antes do workspace utilizável, com instalação/verificação automática, erro e retry, reutilização entre projetos/aberturas e inicialização automática da análise. O drawer precisa virar diagnóstico/reinstalação, sem consentimento experimental ou plano obrigatório.
- A dependência de trust precisa ser conciliada com a ordem de abertura sem enfraquecer o modelo de segurança. A ausência do runtime .NET 10 precisa ter explicação e link adequados; o SDK do projeto continua resolvido sem instalação forçada.

## Evidência pendente

Capturas antes/depois do OpenRidr e testes de primeira e segunda abertura no app desktop real, com `OPENRIDR_HOME` isolado e cópia de OrderDesk. Nenhuma captura das referências proprietárias deve ser copiada ao repositório.
