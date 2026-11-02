# Tickets — QA médios de debug, run e build

Fonte: `docs/SPEC-MEDIUM-DEBUG.md`. Um commit por ticket, local, com `git commit --only -- <paths>`.
Ordem: 01 → 02 → 03 → 04 → 05 → 06 → 07 (05 é só documentação e pode rodar em paralelo, mas o
histórico fica sequencial).

| Ticket | Achado | Entrega |
|---|---|---|
| 01 | d11 | frame do debug reusa aba (caminho normalizado) |
| 02 | d8 | crash com sinal, destaque e pilha clicável |
| 03 | d7 | problemas de compilação da descoberta em Problems |
| 04 | d12 | hover de variável no debug |
| 05 | d10 | decisão + mensagem do debug de teste |
| 06 | b12 | Build de biblioteca no menu |
| 07 | d6 | rótulo "Build startup" sem "Rebuild" |
