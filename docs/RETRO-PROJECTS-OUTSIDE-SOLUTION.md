# RETRO — Projetos fora do diretório da solution (issue #52)

Rodada de 2026-11-06/07, branch `feat/issue-52-projects-outside-solution`: `2c0e19a` (spec + decisão), `3e1dd00` (#143), `7f127e3` (#144), `95fe829` (#145), `8cd7eb8` (#146), `56b75ac` (fixtures), `661793e` (NuGet CPM), `965584b` (evidência), `1260b4a` (merge de `origin/main`), `b0916d3` (correções da revisão), `d077def` (sessão do editor). macOS arm64, SDK `8.0.408`. Spec: [SPEC-PROJECTS-OUTSIDE-SOLUTION](SPEC-PROJECTS-OUTSIDE-SOLUTION.md). Evidência: [evidence/projects-outside-solution.md](evidence/projects-outside-solution.md).

## O que se sustentou

- **Escopo em vez de raiz nova foi a decisão certa.** A árvore, a busca, o cwd de build e o terminal continuaram ancorados na raiz; o que mudou foi só o que o host **pode tocar**. Nenhuma tela precisou de "workspace múltiplo", e a UI só ganhou um selo (`outside`) e caminho absoluto no tooltip.
- **Uma função de confinamento com escopo pagou em todas as fronteiras.** `file.*`, árvore, symlinks, símbolos, LSP, breakpoints, NuGet e problemas de build passaram pelo mesmo `confine` com `&Scope`; a matriz de confinamento (symlink pendente, symlink para dentro, `..` e absoluto de fora incluídos) fechou sem nenhum caso especial.
- **Trust na solution cobrindo o escopo não abriu buraco.** O diálogo declara o alcance antes de confiar, os diretórios vêm do `.sln` confiado (não do usuário) e "uma pergunta por projeto" continua valendo. O teste `trust_model` cobre a decisão por caminho de solution e a captura s1 mostra o texto declarado no diálogo.
- **Restore/build/run/testes do projeto de fora funcionaram de primeira no app real** (s5, s6): `Build succeeded · exit 0` com os dois `.dll` fora da raiz, e o `Run` imprimindo a saída do console de fora.
- **A falha-antes foi medida na base, por teste.** Sete testes de host e o de descoberta falham em `dcb19ea` com motivos distintos (`partial`, `stale: false`, `scope: null`, `outside_workspace`, `unknown`), então cada correção tem um antes e um depois, não um "deve funcionar".

## O que quebrou / custou

- **A revisão independente achou dois altos e cinco médios depois de a suíte estar verde.** `.slnx` no planejamento, texto de trust sem o alcance, outline e `workspace_symbol_items` com caminho relativo para fora, spec do "Show all files", diagnósticos LSP fora do escopo e a seção "App real" vazia na evidência. Verde de teste não cobriu nenhum deles.
- **A sessão morreu duas vezes: 503 do provedor e contexto no teto.** O trabalho sobreviveu porque cada ticket foi commitado; o que se perdeu foi o **texto** da evidência e da retro, que ficou para o handoff. Commit por fatia foi o que salvou a rodada.
- **A sessão do editor (`run_configs`) só apareceu no fim.** O validador de sessão exigia caminho relativo, então a aba de um arquivo de fora — endereçada por absoluto — seria recusada no save da sessão. Ficou como diff não commitado no handoff e virou `d077def` com teste próprio. A lição: o escopo é uma fronteira de **armazenamento**, não só de I/O, e essa não estava na lista.
- **A captura s9 provava o contrário do que o nome dizia.** O `F12` foi apertado com o cursor em `using Lib;` (Ln 1, Col 1); um namespace não tem localização, então "csharp-ls returned no location" era a resposta correta e não dizia nada sobre a fronteira. Refeita com o cursor em `Greeting` → navegou para o arquivo de fora. Captura sem a ação registrada ao lado engana quem lê depois.
- **O csharp-ls não publica diagnóstico de arquivo fora da raiz.** Medido com controle: `Program.cs` (dentro) com `int broken = ;` → `1 error` (CS1525) em ~12 s; `Greeting.cs` (fora) → `0 publications` depois de 2+ min, embora o servidor responda `documentSymbol` e definição para o mesmo arquivo. Ficou como limite com hipótese declarada (filtro por workspace folder), sem investigação nova.
- **Duas rodadas de app para uma feature de host.** Árvore, editor, breakpoint, build, run, edição, diagnóstico e definição só fecharam com o app real; nenhuma delas seria provada por Playwright (decisão da #67).

## Lições

- **Confinamento com escopo é uma lista ordenada, não um conjunto.** A ordem (raiz primeiro) é o que permite descrever o caminho pela entrada mais interna sem inventar regra nova; deduplicar por contenção é o que impede o cap de 64 de virar 64 entradas iguais.
- **Léxico antes de disco continua valendo com escopo.** Relativo com `..` é recusado por forma contra a raiz; absoluto é decidido pela canonicalização contra qualquer entrada do escopo. Misturar as duas coisas é como o oráculo de existência volta.
- **Toda fronteira que guarda caminho precisa do escopo, não só a que lê.** `run_configs` era a que faltava; procurar por `is_absolute` no host teria achado antes.
- **Texto de trust é contrato de segurança.** Se a decisão cobre mais diretórios do que o diálogo diz, o consentimento é inválido — o achado da revisão foi exatamente esse, e o teste não pegaria (é texto).
- **Evidência de app precisa da ação, não só do estado.** A captura prova o que está na tela; sem "o que foi clicado antes" ela pode provar o oposto. Nomear o arquivo pela ação (`s9-definition-…`) não substitui registrar a ação no documento.
- **Controle medido na mesma instância é o que transforma "não funciona" em limite.** Sem o `Program.cs` quebrando de propósito, "0 diagnósticos" seria indistinguível de "o app não pediu nada".
- **Commit por fatia sobrevive a provedor caindo.** Duas sessões morreram; nenhum trabalho foi perdido porque o estado estava no `git log`, não no contexto.

## Follow-ups

- **csharp-ls e diagnóstico de arquivo fora da raiz**: limite declarado, hipótese de filtro por workspace folder. Issue própria se o Helder quiser perseguir (o host não tem o que corrigir: guarda o que chega).
- **`ProjectReference` para projeto fora do `.sln`** não é modelado: o escopo vem dos projetos que o `.sln` nomeia.
- **Busca Files/Text, git status/gutter/diff e cwd do terminal** continuam na raiz; "Show all files" de um projeto de fora lista o diretório do projeto (que está no escopo).
- **Testes (aba Tests) do projeto de fora** provados no host, não clicados no app real nesta evidência.
- Não verificado: Windows, macOS Intel, e o caminho de `.slnx` no app real (só unidade e host).
