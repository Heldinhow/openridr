# Retro — CI em pull requests (issue #56)

Sessão de 2026-11-06. Trabalho: [SPEC-CI-PR](SPEC-CI-PR.md), issues #80 a #88. Retro sobre **o
ambiente de trabalho do agente**, não sobre o produto.

## O que funcionou

- **Medir antes de decidir.** As três perguntas do grill (big bang do `fmt` ou não, quando o check vira
  obrigatório, o que fazer com o security review) só tiveram resposta porque os números existiam:
  596 hunks, 51 linhas de warning, e a suíte do host verde em 4 min. Nenhuma delas foi discutida no
  abstrato.
- **PR mecânico separado do PR de julgamento.** 596 hunks de formatação e 22 tipos de lint dentro de
  um diff de workflow teriam tornado a revisão do YAML impossível. O stack (fmt → clippy → CI) também
  deixou cada commit com o seu próprio "falha-antes": `fmt --check` e `clippy -D warnings` estavam
  vermelhos no `main` e ficaram verdes no commit que os paga.
- **Script + self-check com mutação, no molde da casa.** `scripts/ci-pr-body.sh` é o único código novo
  de verdade, e o self-check de 13 casos provou não ser vazio de duas formas: com um gate falso
  (`exit 0`) ele falha; e com o gate *anterior à revisão* ele falha em
  `issue-inside-code-fence: expected rejected, got accepted`. A segunda mutação é a que importa: ela
  mostra que o aperto do gate foi real, não decorativo.
- **`zizmor` pagou-se na primeira execução.** Antes de existir CI de PR, ele encontrou dois defeitos
  reais no `release.yml` que já estava em `main`: credencial persistida no `.git/config` e cache de
  bun num workflow que assina e publica. Dois comandos de `with:` resolveram.
- **Copiar a convenção do workflow que já existia.** `release.yml` já tinha SHA completo com comentário
  de versão, comentários em inglês explicando o "por quê" e `permissions` mínimo. Seguir aquilo evitou
  inventar estilo e deixou o diff legível.

## O que custou tempo

1. **`cargo clippy --fix` não sai formatado.** O `--fix` reescreve linhas que o `rustfmt` quebraria de
   outro jeito, e eu só descobri no `fmt --check` da bateria final — depois do commit do PR mecânico
   já feito. Custeou `--amend` no commit do clippy e `rebase --onto` no stack. O hook de pre-commit
   roda `bun run check` (tsc + `cargo check`) e não roda `fmt`, então ele não ajuda nesse caso.
   → **Proposta:** incluir `cargo fmt --all -- --check` (e `--manifest-path src-tauri/Cargo.toml`) no
   `.githooks/pre-commit`. Agora que o big bang foi pago, o check custa ~1 s e é exatamente o que o CI
   vai exigir.
2. **Instrução do `AGENTS.md` que envelhece sozinha.** "`check` não inclui fmt/clippy: o código atual
   não passa" era verdade quando foi escrita e virou mentira no commit em que o big bang entrou; nada
   forçava revisá-la. Foi corrigida aqui, mas o mecanismo não existe.
   → **Proposta:** dívida escrita no `AGENTS.md` tem de nomear a issue que a paga e o arquivo onde
   ela será revisitada; fechar a issue fecha a nota no mesmo PR.
3. **Um arquivo, três tickets.** #82, #83 e #84 todos editam `.github/workflows/ci.yml`. Para manter
   um commit por ticket eu escrevi o arquivo em três passes (jobs TS/Rust, depois governança, depois
   segredos). Funcionou, mas é uma decisão que o ticket deveria carregar, não a sessão descobrir no
   meio.
   → **Proposta:** quando N tickets tocam o mesmo arquivo novo, o ticket diz "este arquivo cresce
   incrementalmente na ordem #82 → #83 → #84" (ou o arquivo vira a fronteira de um ticket só).
4. **Descobrir SHA e inputs de action custa caro.** Resolver `v7.0.1` para SHA exige `git/ref/tags` e,
   quando a tag é anotada, um segundo `git/tags/<sha>`; descobrir o nome certo de um input exige ler o
   `action.yml` do repo da action. Foram umas cinco chamadas de `gh api` que cada sessão de CI vai
   repetir.
   → **Proposta:** duas linhas em `docs/ci-main-ruleset.md` (ou num `scripts/` de verdade) com os dois
   comandos: resolver tag→SHA e listar inputs de uma action. Mesmo padrão das "fontes" da pesquisa.
5. **`shellcheck` não existe nesta máquina.** O job de governança liga shellcheck no `actionlint` por
   padrão, mas localmente o `actionlint` roda sem ele: o único check do CI que não consigo reproduzir.
   A sessão do release CI tinha `shellcheck 0.11.0`; esta não.
   → **Proposta:** dizer a versão e o `brew install shellcheck` na seção de CI do `AGENTS.md`, e rodar
   antes do push.
6. **Ruleset não tem dry-run.** O JSON só é validado pela API no momento em que é aplicado; não há
   ensaio. A opção foi aceitar o risco e escrever no doc que a aplicação é do Helder.
   → **Proposta:** aplicar primeiro num repo de teste (ou num branch de teste do ruleset) e só depois
   na `main`, quando o Helder decidir aplicar.
7. **Os runs reais acharam cinco coisas que nenhuma validação local pegaria.** (a) O `zizmor` da
   action audita o repo inteiro (modo de coleta padrão, inclui `dependabot.yml`), enquanto eu rodei
   `zizmor .github/workflows` e vi zero achados: no CI apareceram 4 `dependabot-cooldown` (medium).
   (b) Um teste do host só falha em máquina limpa: `verify_installed` rodava o launcher com
   `current_dir($TMPDIR/openridr-neutral)`, diretório que ninguém cria ali — bug de produto na
   primeira instalação de uma máquina nova (#93). (c) Vários testes restauram NuGet em paralelo numa
   máquina sem `~/.nuget/NuGet/NuGet.Config` → `NU1100`; um `dotnet restore` antes da suíte resolve e
   o caminho do passo errado (`.csproj` que não existe) apareceu no run seguinte. (d) A guarda de
   folhas isoladas da #97 procurava o literal `run_isolated_leaf("`, que o `cargo fmt` quebra em duas
   linhas. (e) Duas asserções exigiam que um poll de 10 ms pegasse o estado transitório `verifying`.
   → **Proposta:** (a) reproduzir a ferramenta com o *mesmo* modo de coleta da action (`zizmor .`, não
   o subdiretório); (b) para teste que depende de `$TMPDIR`, rodar a bateria com `TMPDIR=$(mktemp -d)`
   — é um comando e transforma "passa no Mac" em "passa em máquina limpa"; (c) asserção sobre
   sequência de estados transitórios é asserção sobre a velocidade da máquina — afirme ordem e estado
   final, não a lista completa.
   A regra "check obrigatório só depois do primeiro run verde" se pagou exatamente aqui: durante seis
   runs o Rust esteve vermelho e nenhum deles bloqueou merge.

## Números

- 3 commits originais no PR mecânico (`ci/issue-56-fmt`) e ~20 no PR de CI (`ci/issue-56-pr-checks`,
  incluindo os merges de `main`): um por ticket, mais spec/docs/retro, as correções do code review e
  seis correções que os runs do CI acharam (`cooldown` do Dependabot, #93, restore do NuGet, caminho do
  passo de restore, guarda da #97 e as duas asserções de estado transitório).
- `fmt`: 33 arquivos, 596 hunks; `clippy`: 51 linhas de warning → 0 com `-D warnings`;
  `cargo test -p openridr-host`: 427 testes verdes em duas rodadas (3 min 59 s com build quente, 2 min
  40 s na bateria final); `tsc --noEmit` 0 erros; `bun test tests/unit` 39/39;
  `scripts/ci-pr-body-selfcheck.sh` 13 casos; `actionlint 1.7.12` e `zizmor 1.32.0` (`--min-severity
  low`) sem achado.
- O run final do PR #90 (`2ef7d41`) passou nos cinco checks no runner, ~5,5 min por PR: TS 19 s,
  governance 18 s, security 13 s, Rust 5 min. Não verificado: o `e2e-smoke.yml` (schedule/dispatch,
  nunca rodou), `shellcheck` sobre os `run:`, a aceitação do ruleset pela API e o comportamento real do
  `release.yml` depois das duas linhas (`persist-credentials`, `no-cache`).
