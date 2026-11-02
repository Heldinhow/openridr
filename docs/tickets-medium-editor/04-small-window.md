# 04 — b15: editor continua visível em janela pequena

Status: ready-for-agent
Achado: QA-BUGHUNT-2026-11-02 §15.

## Aceite

- A altura visível do painel inferior nunca rouba o espaço que deixaria o editor abaixo de 180 px (`outputMax` descontando toolbar, splitter e status).
- Em viewport estreita, a largura do painel esquerdo é limitada para preservar o editor e evitar overflow horizontal.
- O usuário continua podendo reabrir/redimensionar os painéis; nada é removido do layout.

## Teste

E2E em 800×520 e 560×420: altura do editor ≥ 170 px, sem `scrollWidth` maior que a viewport, aba do editor visível.
