# SPEC — Tests, NuGet e setup de ferramentas

## Problema e solução

Ao abrir um projeto, a pessoa não deve receber um workspace parcialmente funcional nem precisar instalar e iniciar manualmente ferramentas essenciais. Depois do setup, Tests e NuGet devem apresentar operações reais em interface consistente com as referências, sem copiar assets proprietários.

## Contrato de setup

1. Abrir projeto → verificar/instalar ferramentas com progresso → fluxo existente de trust → workspace. A tela de setup bloqueia o uso do workspace até a verificação terminar; erros são claros e permitem retry.
2. csharp-ls 0.28.0 e netcoredbg 3.2.0-1092 usam origem, versão e checksum fixos; a instalação verificada é compartilhada entre projetos e reutilizada em aberturas posteriores. netcoredbg embutido é habilitado por padrão.
3. Setup de ferramentas fixas não exige trust nem executa código do projeto. Análise, restore, build, test e debug continuam exigindo trust. Após o trust, análise inicia automaticamente; em modo restrito, permanece inativa.
4. Sem runtime .NET 10, explicar o requisito e oferecer o link adequado, sem instalar SDK ou alterar `global.json`. O drawer é diagnóstico, com versões, caminhos, checksums, logs e reinstalação, não um gate manual.

## Tests

- Apresentar barra de ações, filtro, contadores por estado, árvore com estado agregado e detalhe/saída e duração da seleção, mantendo os identificadores existentes.
- Implementar expandir/recolher tudo e exportar resultados; apresentar uma aba de sessão de solução apenas se o modelo de execução puder representá-la fielmente.
- Debug de testes, Continuous Testing e cobertura ficam ausentes ou desabilitados com motivo acessível; nunca simular capacidades inexistentes.

## NuGet

- Apresentar escopo de projeto ou solução, busca, fontes, prerelease, Installed/Discover, contagem e seleção de updates e lista de implícitos/transitivos.
- A solução agrega projetos; implícitos/transitivos vêm de leitura de `obj/project.assets.json`. Uma consulta incompleta ou indisponível deve ser identificada como tal.
- Busca Discover usa API v3 da fonte selecionada. Instalação usa os mesmos gates de plano e confirmação; se não couber, documentar instalação como pendência explícita.
- Fonte e prerelease também afetam consulta de versões disponíveis. Update em lote executa planos por pacote com os gates existentes, informa resultado e falhas parciais por pacote e tenta um restore ao fim sem mascarar falhas. CPM continua recusado.

## Verificação e limites

- Testes de comportamento falham antes e passam depois nas fronteiras existentes de host/UI. Provar que setup sem trust não dispara código do projeto e que operações executáveis continuam bloqueadas.
- Exercitar primeira abertura com home isolado vazio e segunda abertura reutilizando ferramentas, cópia de OrderDesk e app Tauri real via agent-desktop; registrar capturas do OpenRidr e limitações ambientais.
- Não incluir capturas, ícones ou assets do Rider; usar tokens e ícones existentes. Não alterar `DESIGN.md` ou `PRODUCT.md`, não adicionar dependências sem aprovação, não instalar software no sistema. Plataformas além de macOS ARM64 permanecem não verificadas.
