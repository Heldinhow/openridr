# SPEC — Abrir do Finder e do terminal ("Abrir Com") (2026-11-02)

Escopo: declarar associações de `.sln`, `.slnx` e `.csproj` no bundle macOS, receber o arquivo
aberto pelo Finder (app fechado e app aberto), aceitar pasta arrastada para o ícone do Dock e
oferecer `openridr <caminho>` no terminal. Fora: distribuição/notarização, Windows/Linux e
qualquer execução automática ao abrir.

## Problem Statement

Hoje o OpenRidr não aparece em **Abrir Com** no Finder para soluções e projetos C#, e não trata
um arquivo recebido ao abrir. Quem trabalha com o Finder ou com `dotnet` no terminal precisa
abrir o app, voltar para a tela inicial e digitar/colar o caminho — ou pior, acreditar que o
arquivo foi aberto quando o app só ganhou foco.

## Solution

O bundle declara que o OpenRidr abre `.sln`, `.slnx`, `.csproj` e pastas. O arquivo aberto pelo
Finder entra pelo mesmo `workspace.open` da tela inicial: prompt de Trust uma vez por projeto,
`recent.json`, recusa de caminho inválido e guard de buffer sujo continuam valendo. O comando
`openridr <caminho>` faz o mesmo pelo terminal via LaunchServices. Nada é executado ao abrir.

## User Stories

1. Como pessoa que usa Finder, quero ver o OpenRidr em **Abrir Com** para um `.sln`, para abrir a solução sem digitar caminho.
2. Como pessoa que usa Finder, quero poder escolher o OpenRidr como app padrão de `.sln`, para abrir com duplo clique.
3. Como pessoa que usa Finder, quero o mesmo para `.slnx`, para abrir soluções no formato novo.
4. Como pessoa que usa Finder, quero o mesmo para `.csproj`, para abrir um projeto isolado direto.
5. Como pessoa que usa Finder, quero abrir o arquivo com o app fechado, para não precisar abrir o app antes.
6. Como pessoa que usa Finder, quero abrir outro arquivo com o app já aberto, para trocar de projeto sem segunda janela.
7. Como pessoa que usa Finder, quero arrastar uma pasta para o ícone do Dock e abrir a solution/projeto que ela contém.
8. Como pessoa que arrasta uma pasta com duas solutions, quero ver a escolha explícita em vez de o app adivinhar.
9. Como pessoa que abre um projeto desconhecido, quero o prompt de Trust de sempre, para decidir uma vez por projeto.
10. Como pessoa que já confiou no projeto, quero que ele abra direto no modo lembrado, sem novo prompt.
11. Como pessoa que abre um `.csproj` que está dentro de uma solution, quero que ele abra como projeto, igual a escolher o `.csproj` na tela inicial.
12. Como pessoa que abre um `.slnx`, quero ver os projetos da solução na árvore, mesmo que restore/build ainda não sejam suportados pelo SDK do projeto.
13. Como pessoa com buffers modificados, quero que uma abertura vinda do Finder seja recusada com a mensagem existente, para não perder edições.
14. Como pessoa que abriu um arquivo inválido (extensão não suportada, caminho inexistente), quero uma mensagem clara e o workspace atual intacto.
15. Como pessoa que usa terminal, quero `openridr ./Minha.sln` abrindo no app já aberto, para não navegar até o Finder.
16. Como pessoa que usa terminal, quero passar caminho relativo, com espaços ou symlink, e ter o mesmo resultado do caminho absoluto.
17. Como pessoa que usa terminal, quero instalar o link do `openridr` no PATH só quando eu decidir, para o app não alterar meu shell sozinho.
18. Como pessoa que desenvolve o OpenRidr, quero apontar o launcher para um bundle de debug, para testar sem instalar em `/Applications`.
19. Como pessoa responsável pela segurança, quero que abrir um arquivo só leia o necessário para o outline e nunca execute dotnet, script ou build.
20. Como pessoa responsável pela segurança, quero que caminhos fora das extensões/pastas previstas sejam recusados antes de qualquer leitura de projeto.
21. Como pessoa que mantém o app, quero as associações declaradas no bundle, para o macOS mostrar o app em **Abrir Com** sem registro em runtime.
22. Como pessoa que mantém o app, quero o evento nativo (`Opened`) e o argv tratados por uma função pequena e testável, para não duplicar parsing na UI.

## Implementation Decisions

- **Host é a única autoridade de caminho.** `workspace.open` ganha resolução de caminho relativo
  com `baseDir` opcional (absoluto). Ordem: exige `path` não vazio, junta o relativo à base (ou
  cwd), canonicaliza (existência e symlink resolvidos), aceita diretório ou arquivo com extensão
  permitida (`.sln`, `.slnx`, `.csproj`, sem diferenciar maiúsculas), e então carrega. Diretório
  com mais de uma solution/projeto continua devolvendo `ambiguous_workspace` com candidatos.
- **Outline de `.slnx`.** Um parser pequeno lê os elementos `<Project ... Path="..."/>` do XML,
  aceitando aspas simples ou duplas e tag em uma ou várias linhas. O caminho do projeto continua
  canonicalizado e confinado à raiz do workspace, como no `.sln`.
- **Resolução de diretório.** `.sln` e `.slnx` são solutions para efeito de escolha; se houver
  exatamente uma, ela abre; com mais de uma, o erro lista os candidatos. Sem solution, a regra
  atual de um único `.csproj` permanece.
- **Associações no bundle Tauri.** `bundle.fileAssociations` declara os três tipos com
  `exportedType` (identificadores `com.openridr.solution`, `com.openridr.solution-xml`,
  `com.openridr.csproj`), `role: Editor`, `rank: Default` e mime types de texto/XML. Uma quarta
  entrada declara `public.folder` com `rank: Alternate`, para o Dock aceitar pasta sem tornar o
  OpenRidr o padrão para pastas.
- **Recepção no shell.** O processo Tauri lê o primeiro argumento que não é flag (abertura
  direta do binário) e trata `RunEvent::Opened` (LaunchServices: app já aberto ou aberto pelo
  Finder). URL que não é `file://` é ignorada. O caminho fica num estado pendente do processo,
  o último vence, e a UI o consome por um comando Tauri; um evento avisa a UI quando ela já está
  carregada. Nada disso cria janela nova.
- **UI reusa o fluxo existente.** A UI chama `workspace.open` com o caminho pendente e aplica o
  mesmo `handleOpened` (prompt de Trust, `previousPath` para Cancel, stage workspace). Com um
  workspace montado, a abertura externa é encaminhada ao `WorkspaceView`, que usa o mesmo guard
  de `openSolution`/`Open Recent`; buffer sujo recusa e mostra a mensagem existente. Falha de
  abertura aparece numa faixa `role="alert"` dispensável, sem mudar o workspace atual.
- **Launcher.** `scripts/openridr` valida `$# = 1` e a existência do caminho e chama
  `open -a "${OPENRIDR_APP:-OpenRidr}" "$1"`. Não instala link no PATH, não usa a ponte HTTP e
  não abre instância nova. `docs/install-app.md` documenta o link manual e o uso em dev.
- **Sem dependência nova.** Nem Rust nem npm: o parser de XML é local e a integração Tauri usa
  apenas `tauri` já presente.

## Testing Decisions

- **Bom teste = comportamento externo.** O host é exercitado por `dispatch`/ponte HTTP, não por
  funções privadas; o shell é exercitado pela função de argv/URL e pelo bundle gerado.
- **Unidade no host:** caminho relativo com `baseDir`, absoluto, symlink, extensão maiúscula,
  extensão recusada, caminho inexistente, diretório com zero/uma/duas solutions, `.csproj`
  isolado, outline de `.slnx` compacto e multi-linha. Prior art: testes de `workspace.open`,
  `trusted.rs` e `outline.rs` em `crates/host/src`.
- **Unidade no shell:** primeiro argumento ignora flags (`-psn_...`), aceita caminho com espaços;
  `file://` vira caminho local, esquema não-file é ignorado. Ficam em `src-tauri/src/main.rs`,
  compiladas por `cargo test --manifest-path src-tauri/Cargo.toml` (o crate não está no workspace
  raiz, então `bun run check` não o cobre).
- **E2E:** um spec novo chama a ponte HTTP com `workspace.open` e cobre caminho relativo,
  `.slnx`, `.csproj`, diretório ambíguo e extensão recusada, restaurando o workspace ao final.
  Prior art: `tests/e2e/evaluation.spec.ts` e `debugger.spec.ts` chamam o host direto.
- **Manual (não substituto de teste):** build de debug do bundle, inspeção do Info.plist gerado,
  `open --env OPENRIDR_HOME=<.scratch/host-home> -a <bundle> <fixture.sln>` com app fechado e
  aberto, pasta no Dock/`open -a` e o launcher com um `open` falso no PATH. O evento nativo do
  Finder não é automatizável no Playwright; o resultado real vai para a evidência.

## Out of Scope

- Notarização, assinatura de distribuição e instalador público.
- Associações e abertura de arquivo no Windows/Linux (não verificados neste alvo).
- Restore/build/run de `.slnx` com SDK que não o suporta; a falha do dotnet é preservada.
- Editar, criar ou migrar `.sln`/`.slnx`.
- Abrir arquivo arrastado para dentro da janela (só ícone do Dock e LaunchServices).
- Quick Look, preview, "New Window", multiple workspaces simultâneos e single-instance forçado
  fora do comportamento padrão do macOS.
- Instalar o launcher no PATH automaticamente.

## Further Notes

- Rebuild do bundle é obrigatório para o LaunchServices enxergar as associações; um app já
  instalado pode precisar de `lsregister -f` para atualizar o cache do Finder durante a
  verificação manual.
- O `.slnx` entra na árvore e na avaliação por projeto, mas restore/build de solução continuam
  enviando o arquivo ao dotnet; com 8.0.408 o comando falha. Isso é registro honesto, não um
  passe.
- Falhas pré-existentes conhecidas (não introduzidas aqui): find-usages, 2 de run-configs, 2 de
  `tests.spec.ts` e `file_access_is_confined_to_the_workspace` no cargo. Qualquer outra falha
  deve ser reportada.
