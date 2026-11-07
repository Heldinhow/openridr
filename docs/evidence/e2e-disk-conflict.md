# Evidência — conflito de disco determinístico no E2E (#273, ticket #278)

Data: 2026-11-07 (18:38–18:41, UTC−03:00). Ambiente: macOS Apple Silicon, worktree
`t3-c23fba42` sobre `fix/issue-273-workspace-disk-conflict-race`, par de portas **2225 (UI) / 5117
(host)**, app home derivado da porta da UI (`$TMPDIR/openridr-e2e-home-2225`), capturas em
`$TMPDIR/openridr-e2e-screenshots-2225`, `~/.dotnet` no `PATH`, `df -h /System/Volumes/Data` com 25 GB
livres antes da bateria.

Spec: [SPEC-E2E-DISK-CONFLICT](../SPEC-E2E-DISK-CONFLICT.md). Ticket: #278 (filho de #273).
Alvo do teste: `tests/e2e/workspace.spec.ts` › "compares buffer and disk and revalidates a second disk
change before overwriting".

## Causa raiz

O teste escrevia a segunda versão em disco e clicava `conflict-overwrite` ~100 ms depois, enquanto o
`WorkspaceView` re-lê o arquivo ativo num `setInterval` de 1,5 s. Se um tick do poll cair entre a escrita
e o clique, o app atualiza o conflito pendente para a versão nova (comportamento correto), o
`overwriteDisk` revalida contra o **hash novo**, a escrita passa e a segunda versão em disco é
**sobrescrita em silêncio** — o segundo banner não nasce e o assert do conteúdo em disco recebe
`// buffer version`. Com 100 ms de janela contra 1500 ms de tick a corrida é rara (~7% por rodada), por
isso o teste passava quase sempre e falhava por sorte de timing (no runner, run 42412536089, o mesmo
mecanismo apareceu como `toBeVisible` do banner). O fix não mexe no app: o spec segura no bridge
(`page.route("**/__host/call")`) a chamada `file.read` do arquivo ativo, espera o poll bater nessa
leitura, escreve a segunda versão e clica. A leitura segurada nunca chega ao host, então nenhuma resposta
pode substituir o conflito pendente; o `expected_hash` continua o da primeira versão, o host responde
`conflict` e o segundo banner nasce do erro.

## 1. Vermelho — 5/5, com um tick de poll forçado e **sem** o gate

Patch temporário (não commitado): corpo do teste revertido para o pré-spike (`bfb841f^`) e
`await page.waitForTimeout(1600)` entre a escrita da segunda versão e o clique — um tick de poll garantido
dentro da janela.

```sh
export PATH=$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH \
       DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en \
       OPENRIDR_E2E_UI_PORT=2225 OPENRIDR_E2E_HOST_PORT=5117
bunx playwright test tests/e2e/workspace.spec.ts -g "revalidates a second disk change" --repeat-each=5
```

```
  5 failed
    tests/e2e/workspace.spec.ts:365:1 › compares buffer and disk and revalidates a second disk change before overwriting  (×5)
```

As cinco falhas no mesmo ponto (linha 408 do arquivo com o patch):

```
Error: expect(received).toBe(expected) // Object.is equality
- // second disk version
+ // buffer version
> 408 |   expect(fs.readFileSync(program, "utf8")).toBe("// second disk version\n");
```

É exatamente o defeito que o teste existe para impedir: a segunda versão em disco sobrescrita em
silêncio. Duração da bateria: 47 s (18:38:02 → 18:38:49).

## 2. Verde — 5/5, mesmo delay, com o gate do spike

Spike restaurado do commit `bfb841f` (sha256 `c8ff436d72c2…`), delay de 1,6 s mantido no mesmo lugar:

```sh
bunx playwright test tests/e2e/workspace.spec.ts -g "revalidates a second disk change" --repeat-each=5
# 5 passed (36.5s)
```

Cada rodada levou 6,1 s (contra 4,4 s sem delay), ou seja, o delay estava de fato dentro da janela e o
poll bateu no gate nas cinco.

## 3. Verde — 20/20, sem delay

Delay removido; o arquivo voltou a ser byte a byte o do spike (`git status` limpo, sha256
`c8ff436d72c2…`):

```sh
bunx playwright test tests/e2e/workspace.spec.ts -g "revalidates a second disk change" --repeat-each=20
# 20 passed (1.7m), 4,4–4,5 s por rodada
```

| Medida | Resultado |
|---|---|
| sem gate + delay 1,6 s, `--repeat-each=5` | **5 failed**, todas em `fs.readFileSync` do conteúdo em disco (`+ // buffer version`) |
| com gate + delay 1,6 s, `--repeat-each=5` | **5 passed** (36,5 s; 6,1 s por rodada) |
| com gate, sem delay, `--repeat-each=20` | **20 passed** (1,7 min; 4,4–4,5 s por rodada) |

## O que o gate acrescenta (por que não pode ser removido em silêncio)

`holdDiskRead` devolve `untilHeld()`, e o teste espera a leitura entrar no gate **antes** de escrever a
segunda versão. Sem essa espera o teste voltaria a passar por sorte de timing (a janela é ~7% do tick);
com ela, remover o gate derruba o teste sempre — o vermelho do item 1 é essa prova. O release fica no
`finally` e o `unroute` só acontece depois que o handler terminou (senão o `route.continue()` estoura
`Route is already handled!`, medido na sessão do spike).

## 4. Regressão da área e pós-merge (sessão mãe, integration branch)

Depois do merge do ticket e antes do merge de `main`:

```sh
bunx playwright test tests/e2e/workspace.spec.ts
# 16 passed (56.4s) — o teste alvo levou 4,2 s (2,5 s antes do gate: o untilHeld espera até um tick)
```

Depois do merge de `origin/main` (`2265461`, que trouxe #265, #276 e #251/#277) e da resolução dos
conflitos append-only de `GLOSSARY.md`/`docs/README.md`/`docs/decisions.md`:

```sh
bunx playwright test tests/e2e/workspace.spec.ts -g "revalidates a second disk change" --repeat-each=8
# 8 passed (1.0m)
```

## O que ficou sem prova

- **Runner do GitHub**: nada disso foi medido no CI; as três baterias são locais, na máquina acima. O
  smoke noturno é quem vai exercitar o spec no runner.
- **`tests/e2e/workspace.spec.ts` inteiro**: não rodado por esta sessão (é a bateria final da sessão
  mãe), então a regressão do resto do spec não está provada aqui.
- **Windows e macOS Intel**: não executados, como no resto do projeto.
