# Dependências não listadas: declarar `@lezer/highlight`, `@codemirror/search` e `@codemirror/autocomplete`

Branch: `fix/issue-223-unlisted-deps` (base `f9c3da5`). Data: 2026-11-07. Issue #236, mãe #223.
Escopo: `package.json` e `bun.lock`. Nenhuma linha de `src/` tocada.

## Resumo

| Verificação | Antes | Depois |
|---|---|---|
| `bun install --frozen-lockfile` | **passa** (ver ressalva abaixo) | passa |
| Tabela `workspaces."" dependencies` do `bun.lock` | **faltavam os três** | os três presentes |
| `bun run build` | — | passa |
| `bun run check` | — | passa |
| Diff do `bun.lock` | — | `3 +++`, só a tabela do workspace raiz |

**Ressalva sobre o failing-first, medida e não presumida:** o critério de aceite literal do ticket
("`bun install --frozen-lockfile` falha antes de mexer no lock") **não se reproduz** no bun 1.3.9, que é a
versão pinada nos workflows (`bun-version: "1.3.9"` em `ci.yml`, `e2e-smoke.yml`, `release.yml`,
`dependency-audit.yml`). O `--frozen-lockfile` do bun 1.3.9 só falha quando o **conjunto resolvido** de
pacotes mudaria; uma tabela de dependências do workspace desatualizada passa em silêncio, porque os três
pacotes já estão na tabela `packages` como transitivos de `codemirror`. Prova do mecanismo, num diretório
descartável:

```
$ # baseline: só codemirror declarado
$ bun install --frozen-lockfile ; echo exit=$?
exit=0
$ # declara @lezer/highlight, que já está em packages por transitividade
$ bun install --frozen-lockfile ; echo exit=$?
Checked 16 installs across 17 packages (no changes) [2.00ms]
exit=0
$ # declara um pacote que NÃO está no lock
$ bun install --frozen-lockfile ; echo exit=$?
error: lockfile had changes, but lockfile is frozen
note: try re-running without --frozen-lockfile and commit the updated lockfile
exit=1
```

O critério continua valendo como contrato (manifesto e lock concordam no fim), mas ele não é o que
detecta a lacuna: quem a detectou foi o diff do lock. A prova substituta, abaixo, é o estado exato da
tabela do workspace antes e depois.

## Antes

`package.json` não declarava os três, e a tabela do workspace no lock também não — os três só existiam em
`packages`, como transitivos de `codemirror`:

```
$ grep -n '"@codemirror/search"\|"@codemirror/autocomplete"\|"@lezer/highlight"' bun.lock | head
36:    "@codemirror/autocomplete": ["@codemirror/autocomplete@6.20.4", "", { ... } ],
46:    "@codemirror/search": ["@codemirror/search@6.7.2", "", { ... } ],
60:    "@lezer/highlight": ["@lezer/highlight@1.2.5", "", { ... } ],
180:    "codemirror": ["codemirror@6.0.2", "", { "dependencies": { ..., "@codemirror/search": "^6.0.0", ... } } ],
```

As três versões que o lock já resolvia — 6.20.4, 6.7.2, 1.2.5 — são exatamente as que a faixa `^` declara.

## Mudança

`package.json`, em `dependencies`, na ordem alfabética do bloco:

```diff
   "dependencies": {
+    "@codemirror/autocomplete": "^6.20.4",
     "@codemirror/commands": "^6.8.1",
     "@codemirror/language": "^6.11.0",
     "@codemirror/legacy-modes": "^6.5.1",
+    "@codemirror/search": "^6.7.2",
     "@codemirror/state": "^6.5.2",
     "@codemirror/view": "^6.36.5",
     "@fontsource-variable/inter": "^5.3.0",
     "@fontsource-variable/jetbrains-mono": "^5.3.0",
+    "@lezer/highlight": "^1.2.5",
     "@tauri-apps/api": "^2.5.0",
```

`bun install` (bun 1.3.9) gerou o lock. Diff **inteiro** do `bun.lock`:

```diff
@@ -5,13 +5,16 @@
     "": {
       "name": "openridr",
       "dependencies": {
+        "@codemirror/autocomplete": "^6.20.4",
         "@codemirror/commands": "^6.8.1",
         "@codemirror/language": "^6.11.0",
         "@codemirror/legacy-modes": "^6.5.1",
+        "@codemirror/search": "^6.7.2",
         "@codemirror/state": "^6.5.2",
         "@codemirror/view": "^6.36.5",
         "@fontsource-variable/inter": "^5.3.0",
         "@fontsource-variable/jetbrains-mono": "^5.3.0",
+        "@lezer/highlight": "^1.2.5",
         "@tauri-apps/api": "^2.5.0",
```

```
$ git diff --stat package.json bun.lock
 bun.lock     | 3 +++
 package.json | 3 +++
 2 files changed, 6 insertions(+)
$ git diff -U0 bun.lock | grep -c '^[+-].*\[".*@'   # linhas da tabela `packages`
0
```

Nenhuma entrada nova em `packages`, nenhuma versão nova, nenhum pacote novo. Os dois
`@fontsource-variable/*` continuam declarados (decisão de 2026-11-07 em `docs/decisions.md`: são usados por
`url()` em `src/styles.css` e o `knip` não segue `url()` de CSS).

## Depois

```
$ export PATH="$HOME/.bun/bin:$PATH"
$ bun install --frozen-lockfile
bun install v1.3.9 (cf6cdbbb)

Checked 55 installs across 109 packages (no changes) [14.00ms]
EXIT=0
```

```
$ bun run build
$ tsc --noEmit && vite build
vite v8.3.5 building client environment for production...
✓ 2160 modules transformed.
dist/assets/index-BE--nZT5.js                                1,452.77 kB │ gzip: 413.96 kB
✓ built in 473ms
EXIT=0
```

O aviso de chunk acima de 500 kB já existia e não vem desta mudança (nenhuma linha de `src/` foi tocada).
O `vite build` é a única prova que existe para `tags`/`@lezer/highlight`, porque nenhum spec asserta classe
de token.

```
$ bun run check
$ tsc --noEmit && cargo check --workspace --all-targets
    Checking openridr-host v0.3.0-alpha.11 (.../crates/host)
    Checking openridr-host-http v0.3.0-alpha.11 (.../crates/host-http)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.26s
EXIT=0
```

## Não verificado

- **`tests/e2e/editor-find-count.spec.ts` em modo web** (critério de aceite do ticket, prova de runtime do
  `@codemirror/search`): não rodado nesta sessão por instrução explícita de escopo. É a prova que falta
  para fechar o comportamento de busca/contagem/goto-line com o pacote declarado; o `vite build` prova só
  que ele empacota.
- **`@codemirror/autocomplete` em runtime**: nenhum spec cobre o autocomplete do editor, e esta issue não
  criou um. A cobertura de runtime dele segue sendo nenhuma, antes e depois da mudança.
