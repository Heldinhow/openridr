# SPEC — Diretório neutro da ferramenta em um lugar só (issue #93)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe: [#93](https://github.com/Heldinhow/openridr/issues/93), achada pela primeira execução do workflow de PR (#90).

## Problema

Toda verificação de ferramenta roda o launcher com `current_dir($TMPDIR/openridr-neutral)` — um diretório fora do workspace, para o launcher não enxergar projeto nenhum. Numa máquina fria esse diretório não existe, e `current_dir` de caminho inexistente faz o **spawn** falhar com `ENOENT` **antes** do launcher rodar: a ferramenta é reportada como inválida (`Could not run <staging>/csharp-ls --version.`) em vez de verificada. É o primeiro install de uma máquina nova; na máquina de quem desenvolve o diretório já existe, então o bug é invisível.

O `#90` já corrigiu o sintoma em `verify_installed` (o mesmo PR que descobriu o bug), mas o passo "criar o diretório neutro" ficou **duplicado em três lugares** — `verify_installed`, `verify_bundled` e o worker de install — cada um com a sua forma de tratar o erro. Quem mexer em um deles não é avisado dos outros: é a mesma armadilha que gerou o bug, um `create_dir_all` esquecido de distância.

## Solução

O diretório neutro passa a ser criado **na função que o nomeia**, `neutral_cwd()`, e os três chamadores perdem o passo próprio. Não existe mais "lembrar de criar": quem pede o caminho recebe um caminho que existe.

Um teste determinístico prova a criação a partir de um diretório ausente, independente do `TMPDIR` estar quente ou frio — o teste que já existe só reprova em runner limpo, que é justamente o que escondeu o bug até o CI.

## User stories

1. Como pessoa que instala a ferramenta numa máquina nova, quero que a verificação rode o launcher em vez de reportar "Could not run", para o primeiro install terminar verificado.
2. Como pessoa que instala a ferramenta, quero que a falha de criar o diretório neutro não vire uma mensagem que fala de executar o binário, para eu entender o que de fato falhou.
3. Como pessoa que mantém o OpenRidr, quero um lugar único que garante o diretório neutro, para não haver um segundo ponto para esquecer.
4. Como pessoa que mantém o OpenRidr, quero um teste que reprove na minha máquina quente se o passo for removido, para a regressão não depender de um runner limpo.
5. Como pessoa que mantém o OpenRidr, quero a falha-antes medida com `TMPDIR` fresco, para saber que o teste é guarda e não coincidência.
6. Como pessoa que revisa o CI, quero a suíte de host verde num runner limpo, para o caminho de máquina fria ficar coberto.
7. Como pessoa que usa o app, quero que nada mude na UI nem no plano de instalação, porque o problema é só de host.

## Implementation decisions

- **A criação mora em uma função só, e só ela** (decisão do grill, Q2a). `neutral_cwd()` continua sendo só o caminho — é o que o plano anuncia —, e `neutral_cwd_ready()` cria o diretório e devolve o caminho, ignorando o erro de criação (o mesmo tratamento que `verify_bundled` já dava). `verify_installed`, `verify_bundled` e o worker de install chamam a segunda e perdem o `create_dir_all` próprio. A separação existe porque o plano é chamada de leitura: a primeira versão criava dentro de `neutral_cwd()` e o plano passou a escrever no `$TMPDIR` antes de qualquer confirmação (achado da revisão independente).
- **Custo declarado:** o worker de install perde a mensagem específica "Could not create the neutral install directory". Só aparece com `TMPDIR` não gravável, e nesse caso o spawn falha logo depois com `ENOENT` e a mensagem de spawn. Aceito: um lugar único vale mais que uma mensagem melhor num caso raro, e o caminho que a issue pede (máquina fria, `TMPDIR` gravável) não passa por ali.
- **O plano continua anunciando o caminho, não criando nada.** `plan.cwd` é o texto do diretório neutro no plano de instalação, vindo de `neutral_cwd()`; quem cria é o primeiro spawn, via `neutral_cwd_ready()`.
- **Prova determinística** (Q3a): a criação é extraída para `neutral_cwd_in(root)`, com `neutral_cwd()` chamando com `std::env::temp_dir()`. O teste usa um `tempdir` próprio e assere que o diretório **não existia** e existe depois. Sem mexer em `TMPDIR` do processo (mutação de ambiente em teste paralelo é corrida).
- **Sem dependência nova, sem UI, sem `data-testid`, sem mudança de contrato.** Nenhum `data-testid` novo ou renomeado; o payload do plano e as mensagens de verificação ficam iguais, exceto pela mensagem de criação do install que deixa de existir.
- **Verificação sem app real** (Q4a): a mudança é de host, sem superfície de UI. O app real entra como **não verificado**, com o motivo.

## Testing decisions

- **Um teste bom aqui prova o efeito no disco**, não o texto de uma mensagem: o diretório neutro existe depois de pedido, partindo de um pai onde ele não existe.
- **Seam:** `neutral_cwd_in(root)` — a mesma função que os três chamadores usam, chamada com um `tempdir` em vez de `std::env::temp_dir()`. Nenhum seam novo de produção.
- **Falha-antes / passa-depois:** com a criação removida, `TMPDIR=<fresco> cargo test -p openridr-host --lib tools::tests::staged_verification` reprova com `Could not run ... --version.`; com a correção, passa. É a reprodução da issue, medida de novo.
- **Guarda que não depende de runner limpo:** o teste de `neutral_cwd_in` reprova em qualquer máquina se a criação sair da função.
- **Prior art:** os testes de `tools::tests` já usam `tempfile::tempdir()` para staging e destino; o teste novo segue o mesmo molde.
- **Suíte completa** `cargo test -p openridr-host` uma vez no fim, além de `cargo fmt` e `clippy -D warnings` (o CI roda os dois).

## Out of scope

- Verificação no app Tauri real: sem superfície de UI, e um install de verdade exige download do NuGet. Registrado como não verificado.
- Windows e macOS Intel.
- Trocar o diretório neutro por outro caminho, ou limpá-lo ao sair.
- Qualquer mudança no fluxo de install, no plano, nas mensagens de verificação ou na UI.

## Further notes

- Evidência em `docs/evidence/staged-tools-neutral-dir.md`; decisão em `docs/decisions.md`; linha em `docs/README.md`; retro em `docs/RETRO-STAGED-TOOLS-NEUTRAL-DIR.md`.
- O teste que a issue cita (`staged_verification_writes_integrity_and_a_write_failure_keeps_the_old_copy`) não muda: ele já é a reprodução quando roda com `TMPDIR` fresco.
