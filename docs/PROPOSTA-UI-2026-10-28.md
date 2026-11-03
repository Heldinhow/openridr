> **Histórico — não é o estado atual do produto.** Proposta de UI de 28/10/2026 (antes de qualquer implementação). O texto abaixo, inclusive "Ponto de parada: aguardando aprovar as TELAS" e "nenhum produto implementado", está superado: o app existe (alpha.4). Estado atual: [README.md](README.md) e [MAPA.md](MAPA.md).
# OpenRidr — revisão de UI e MVP

**Proposta estática / controles ilustrativos.** Planejamento apenas: nenhum produto implementado, instalação executada ou compatibilidade comprovada por estes desenhos.

- [index.html](index.html): sete telas offline, âncoras reais e estados alternativos visíveis; sem JavaScript ou dependências.
- [wireframes.md](wireframes.md): as mesmas sete telas em ASCII, fontes de dados e regras de comportamento.
- Este documento: decisões preservadas, recorte proposto e aceitação futura.

## Decisões do handoff — não estão em votação

- **Mac e Windows no MVP**, código aberto. OpenIdea (Apache-2.0, Tauri + Rust + TypeScript, keymap IntelliJ) é base/inspiração; não presumir que toda capacidade do upstream já está integrada.
- **.NET 8+ suportado; net6 best effort.** SDK .NET 10 obrigatório para análise com **csharp-ls 0.28**. O host de análise não é o SDK selecionado para o projeto, nem o seu TFM. Roslyn LS fica para depois.
- **netcoredbg 3.2.0** para debug. macOS ARM64 fica atrás de feature flag experimental **OFF por padrão**, sem bloquear release. Habilitar a experiência requer consentimento separado da instalação. Issue upstream **#256** é risco de reporte de saída/stop, **não falha reproduzida neste trabalho**. Sem hot reload.
- Não usar vsdbg, C# Dev Kit, Rider/ReSharper ou branding JetBrains. OpenRidr mantém identidade própria.
- **UI em inglês desde v1; documentação em português.** Keymap IntelliJ/Rider por padrão: label `Keymap: IntelliJ / Rider`, compatibilidade de atalhos, não backend/componente proprietário. Sem seletor extra. Run: Mac Ctrl+R / Windows Shift+F10; Debug: Mac Ctrl+D / Windows Shift+F9; breakpoint: Mac Cmd+F8 / Windows Ctrl+F8; step over F8 / step into F7.
- **O app instala ferramentas**, por download ou `dotnet tool`, após confirmação explícita do plano e com log visível. Isso não autoriza instalar SDK de sistema.
- **Runner básico genérico VSTest no P0**, incluindo xUnit via adapter instalado no projeto; não exclusivo de xUnit nem promessa de todos os frameworks/adapters. MTP fora de P0.

## Propostas desta revisão — aguardam Helder

Somente a apresentação das telas aguarda aprovação: tema escuro integral, cobre para seleção, cores semânticas com texto; sans para interface e mono somente para código/logs. As strings são exemplos, não relatórios de uma execução. As seis decisões acima estão aprovadas, não são perguntas reabertas.

Fixture de planejamento: `OrderDesk.sln`, console `OrderDesk.Cli`, biblioteca `OrderDesk.Core` e testes `OrderDesk.Tests`, todos `net8.0`. `global.json` fixa `8.0.408` com `rollForward: disable`; o projeto usa esse SDK instalado em paralelo ao SDK 10 do host de análise. Não alterar `global.json` para satisfazer o LSP. A compatibilidade dessa combinação com o carregamento do csharp-ls deve ser provada; se falhar, mostrar log e manter edição de texto, sem migrar o projeto silenciosamente.

### Segurança e sequência

1. Inspecionar texto de solução/projeto/`global.json` e inventário global de ferramentas sem executar código do repositório. Detecção de SDKs usa contexto neutro, fora do workspace.
2. Mostrar **Restricted mode**, caminho completo e confiança **desmarcada**. Não avaliar MSBuild, iniciar csharp-ls, restaurar, executar tarefas, abrir PTY no repositório ou iniciar build/debug antes da confiança explícita.
3. Após confiar: resolver SDK do projeto → avaliar MSBuild → restore solicitado explicitamente → iniciar/carregar análise → build/run/debug por ação do usuário. Falhas não retiram acesso à edição de texto.
4. Confiança revogada bloqueia novas execuções e encerra serviços do workspace; processos ativos exigem confirmação de encerramento. Instalação/download exige consentimento próprio; confiar no repositório não autoriza instalar ferramentas.

### Instalação de ferramentas, não do SDK

O plano prévio identifica ferramenta, versão, fonte, destino, comando/operação exatos e permissões. Exemplos de proposta: `dotnet tool install csharp-ls --version 0.28.0 --tool-path <app-tools>/csharp-ls/0.28` via NuGet.org; download de netcoredbg 3.2.0 da release oficial Samsung/netcoredbg para diretório de ferramentas do app. O destino é fora do repo, sem `--global`; resolver caminho absoluto e asset OS/arquitetura antes de permitir confirmação. Sem asset compatível verificado, instalação fica bloqueada — não prometer asset Mac Intel 3.2 ou compatibilidade por arquitetura ainda não comprovada.

Após confirmar: downloading → installing → verifying, com cancelamento, falha e retry explícitos; preservar stdout/stderr e log de download/verificação. Percentual só quando fornecido pela fonte; verificar versão e execução antes de declarar sucesso (isso não prova DAP). SDK .NET 10 ausente: orientar instalação oficial e nova detecção; app não inclui instalador SDK de sistema neste plano e nunca altera `global.json`. Consentimento de instalação, trust do repo e habilitação experimental são três decisões independentes.

## Prioridades propostas

P0 é um ciclo pequeno e completo **nos dois sistemas**, não um IDE maduro. Dependências vêm antes dos consumidores.

| Prioridade | Capacidade | Justificativa e prova futura |
|---|---|---|
| P0.1 | Trust, toolchain e instalação confirmada | Restrito abre texto sem execução; SDK ausente, host 10 e SDK de `global.json` têm mensagens/ações distintas. App instala ferramentas com plano, confirmação, log, cancelamento e verificação; SDK por orientação oficial, não instalador automático. |
| P0.2 | Uma solução e árvore avaliada | Projetos, TFM, referências e pacotes; `bin/obj` ocultos. Avaliação somente após trust; projeto com falha permanece na árvore. DLL de debug vem do `TargetPath` avaliado para Debug/net8.0. |
| P0.3 | Edição e LSP essencial | Completion, definição, rename e diagnósticos; code actions apenas quando retornadas pelo LSP no contexto. Log/reinício do serviço e indicação de cobertura incompleta. Rename entre console/lib com prévia. |
| P0.4 | Startup + build/run + painel Build | Seletor explícito de executável, configuração e TFM já no P0: o mockup depende deles. Output navegável, comando/log bruto preservado, cancelar e exit code quando conhecido. Sem perfis avançados. |
| P0.5 | Debug launch básico, condicionado à plataforma | Build antes de launch, breakpoint verificado/pendente, continue/step/stop, frames e locals. ARM64 Mac experimental OFF: Debug desabilitado com explicação, Run disponível. Sem asset/adaptador compatível, bloquear debug nessa arquitetura. Stop sem exit reportado mostra desconhecido. |
| P0.6 | Um terminal PTY | Shell/cwd explícitos, entrada e Ctrl+C, confirmação ao fechar shell ativo. `dotnet test` manual continua possível; não inferir exit do comando a partir da vida do shell. |
| P0.7 | Test Runner básico VSTest genérico | Seleção de projeto/testes, descoberta via adapter instalado no projeto, todos/selecionados, cancelamento, passed/failed/skipped, stack e navegação quando posição fornecida; stdout/stderr e fallback em logs. xUnit via adapter VSTest do projeto; nunca instalar/alterar packages silenciosamente. |
| P1 | Uso diário | Testes avançados (cobertura/gutters/debug test, a priorizar), perfis/`launchSettings.json`, referências/usages e format, settings ampliadas e busca de símbolos. |
| P2 | Expansão | MTP a avaliar, attach, conditional/log breakpoints, watches/evaluate e políticas de exceção avançadas; NuGet UI, templates, multi-target avançado e splits/abas de terminal. Não duplicar isso em P0. |

Sem hot reload; sem designers WPF/WinForms, profiling, .NET Framework legado, Razor/Blazor completo, desenvolvimento remoto ou IA embutida neste recorte. Não prometer paridade de recursos de ferramentas comerciais.

Runner básico P0 tem impacto **pequeno/médio em relação ao CLI**, mas é trabalho novo: descoberta, modelo de resultados e navegação não são integração herdada comprovada. Justifica-se por fechar o ciclo editar/build/testar sem copiar comandos; não inclui cobertura, gutters, debug de teste ou MTP. VSTest/TRX é proposta técnica a validar, com logs preservados quando faltarem dados estruturados. Projeto MTP mostra exatamente: `Microsoft.Testing.Platform is not supported in this version. Use the terminal.` Não tentar comandos VSTest sobre MTP nem modificar o projeto para convertê-lo.

## Diagnósticos: contrato de apresentação

- **Origem**: `LSP · csharp-ls` ou `Build · dotnet`. **Categoria** é outro campo: `Compiler`, `Analyzer` ou `Unknown` quando a fonte não a identifica. Nunca usar “LSP” como categoria de regra.
- Exemplo falho tem três itens: CS0246 **error / compiler / LSP**, CS8618 **warning / compiler / build**, CS0168 **warning / compiler / build**. Origem continua visível mesmo quando o mesmo código vier de duas fontes; não mesclar snapshots silenciosamente.
- Quick fix depende de `textDocument/codeAction` e da posição/documento atuais, não do código CS por si só. Mudança aplicada vira “Awaiting diagnostics”; não riscar como corrigido antes de republicação. Build anterior continua rotulado com sua execução/revisão.
- `dotnet build` não fornece JSON global por promessa. Proposta: captura de texto e parser conservador para posições conhecidas, sempre preservando saída bruta; logger próprio só se a validação demonstrar necessidade.
- “No problems reported” exige escopo, fonte e revisão verificados. LSP não oferece contador global/conclusão universal de solução: publicação vazia de um documento só prova ausência de itens publicados para esse documento. Análise carregando, restore incompleto ou arquivo modificado não podem virar “solução limpa”.

## Aceitação futura por plataforma (nada executado aqui)

Executar a mesma fixture net8 em **macOS Intel, macOS Apple Silicon e Windows x64**, registrando versões, arquitetura, comandos e logs. Mac e Windows saem juntos. Run/análise devem ser validados separadamente de debug e da disponibilidade de assets; não tratar presença de binário como teste DAP.

| Prova | Mac | Windows |
|---|---|---|
| Restrito | Confirmar ausência de avaliação MSBuild, restore, LSP e processos do workspace antes de confiar | Mesma prova, inclusive caminhos com espaços e políticas de execução do shell |
| Dois SDKs | Host 10 identificado separadamente do SDK 8.0.408 de `global.json`; SDK faltante não altera o arquivo | Mesma prova e resolução correta do `dotnet`/PATH |
| Árvore + análise | Console/lib/testes; restore incompleto e projeto falho legíveis; completion/definição/rename entre projetos | Mesma sequência, paths case-insensitive e posições de diagnóstico corretas |
| Build + run | Erro clicável, correção, rebuild sem erro, saída do console; cancelamento e processo encerrado verificáveis | Mesma sequência, CRLF e encerramento da árvore de processos |
| Instalação | Plano confirmado, download/dotnet tool, log, cancel/failure/retry e verificação; sem instalar SDK de sistema | Mesma prova, caminhos e permissões Windows |
| Debug real | Intel: só com asset compatível verificado; sem asset, indisponível explícito. ARM64 OFF por padrão, Run segue; experiência opcional não bloqueia release | Smoke DAP com adaptador adequado à arquitetura, launch/breakpoint/step/stop |
| Runner + PTY | Descoberta VSTest/xUnit, todos/selecionados, passed/failed/skipped, stack/posição quando disponível, logs; cancel, vazio, adapter ausente, falha e MTP; Ctrl+C no PTY | Mesma prova com shell/PTY Windows; não confundir exit de shell com comando |

A flag ARM64 só pode ser retirada após evidência em Mac arm64 usando **`netcoredbg --interpreter=vscode` (DAP)**: launch, breakpoint, step e stop, incluindo async e multi-thread. Guardar logs e comprovar encerramento; #256 permanece risco não reproduzido. Habilitar experimental não comprova compatibilidade nem dispensa esse gate.

SDK ausente/restauração quebrada, build falho, breakpoint pendente, adaptador ausente e encerramento sem código também são casos de aceitação. Para net6, registrar limites como best effort, nunca equivalência ao suporte 8+. Outros TFMs .NET 8+ requerem SDK/runtime apropriados e evidência adicional; a fixture net8 não prova todos os alvos.

## O que mudou nesta revisão

- Seis decisões aprovadas separadas da apresentação pendente: inglês v1, instalação pelo app, Mac/Windows juntos, ARM64 experimental OFF, runner VSTest P0 e keymap IntelliJ/Rider.
- Sete telas: novo Test Runner visual e ASCII, resultados/stack, descoberta, vazio, adapter ausente, falha, cancelamento e MTP não suportado.
- Instalação com plano, confirmação e logs/estados; SDK continua orientação oficial, separado de tools e sem mudar global.json.
- ARM64 não aparece pronto por padrão: Debug bloqueado, Run independente; gate DAP/async/multi-thread e assets por arquitetura sem promessa Mac Intel.
- Runner P0 é novo trabalho pequeno/médio frente ao CLI; recursos avançados permanecem P1/P2. Identidade OpenRidr e atalhos concretos preservados.
- Estilo escuro, navegação offline acessível, controles desabilitados e estados sem progresso inventado preservados.

## Ponto de parada

**Aguardando Helder aprovar as TELAS.** Decisões aprovadas não estão reabertas. Não criar tickets, implementar, instalar ferramentas ou avançar automaticamente.
