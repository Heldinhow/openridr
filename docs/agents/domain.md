# Domain docs

Como as skills de engenharia devem consumir a documentação de domínio deste repo antes de explorar o código.

Layout: **single-context** — um `GLOSSARY.md` e um log de decisões na raiz, sem `GLOSSARY-MAP.md` nem ADR por contexto.

## Antes de explorar, leia

- **`GLOSSARY.md`** (raiz) — vocabulário do produto: Solution Explorer, Run configuration, Build implícito, Trust decision, Restricted mode, etc.
- **`docs/decisions.md`** — o equivalente de `docs/adr/` neste repo: decisões incrementais em ordem cronológica (append). `grep -n '^## ' docs/decisions.md | grep -i <tema>` para achar; entradas marcadas "Substituída…" já não valem.
- **`docs/README.md`** — índice: feature → spec / ticket / retro / evidência, e o que é histórico (não use `docs/PROPOSTA-UI-*`, `release-decision.md`, `index.html`, `wireframes.md` como estado atual).
- **`docs/MAPA.md`** — onde fica o quê no código; é o mapa de navegação antes de abrir `WorkspaceView.tsx` ou `crates/host/src/lib.rs`.

## Estrutura

```
/
├── GLOSSARY.md
├── AGENTS.md
├── docs/
│   ├── decisions.md        ← decisões (ADR-style, append)
│   ├── README.md           ← índice dos docs
│   ├── MAPA.md             ← mapa do código
│   ├── SPEC.md             ← contrato do produto (P0)
│   ├── SPEC-<tema>.md
│   ├── RETRO-<tema>.md
│   ├── evidence/<tema>.md
│   └── agents/             ← esta config
├── src/
├── crates/
└── src-tauri/
```

`docs/tickets-*/` existe para ondas antigas e é **histórico**; ticket novo é GitHub issue (ver `issue-tracker.md`).

Arquivos que não existem (ex.: `docs/adr/`) não devem ser criados de reflexo, e a ausência deles **não** é aviso: a skill `/domain-modeling` (via `/grill-with-docs` e `/improve-codebase-architecture`) cria vocabulário e decisão quando — e só quando — um termo ou uma decisão realmente é resolvida.

## Use o vocabulário do glossário

Ao nomear um conceito de domínio (título de issue, proposta de refactor, hipótese, nome de teste), use o termo como o `GLOSSARY.md` define. Não derive para sinônimos que o glossário evita.

Se o conceito que você precisa ainda não está no glossário, isso é sinal: ou você está inventando linguagem que o projeto não usa (reconsidere), ou há lacuna real (anote para `/domain-modeling`).

## Sinalize conflito com decisão

Se o seu output contradiz uma decisão de `docs/decisions.md`, diga isso explicitamente em vez de sobrescrever em silêncio:

> _Contradiz a decisão de 2026-11-04 (confinamento de arquivos), mas vale reabrir porque…_
