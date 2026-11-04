# 13 — Adiado: debug de teste com attach ao test host (G10)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Out of Scope". **Bloqueado por:** nenhum para
documentar; a implementação depende de suporte novo. **Status:** adiado.

## O que falta

Depurar um teste unitário até um breakpoint, como o Rider faz ao vivo: o processo de teste
(`vstest`/test host) precisa ser identificado, o netcoredbg precisa anexar a ele, e a sessão
DAP precisa correlacionar o resultado do teste com os frames. Hoje o OpenRidr só lança um
processo de aplicação; não há attach a processo existente nem descoberta do PID do test host.

## Estado atual (não muda nesta onda)

- "Debug selected" continua desabilitado com o motivo visível ("needs attach to the test host,
  which is not supported").
- Nenhum caminho falso de debug de teste é habilitado.
- A decisão da grill registra o adiamento e o motivo.

## O que destrava

1. Identificar o processo do test host e o momento seguro para anexar.
2. Implementar attach DAP no netcoredbg (ou um caminho equivalente) e correlação com a sessão
   de teste.
3. Mapear os frames do teste para arquivo:linha e manter o resultado da execução.
4. Provar com um teste que para num breakpoint e mostra threads, pilha e variáveis.

## Aceite (deste ticket de registro)

- [ ] O motivo do "Debug selected" continua visível no painel Tests.
- [ ] A decisão de adiamento e o que falta estão em `docs/decisions.md` e neste ticket.
- [ ] Nenhuma capacidade de debug de teste é anunciada como disponível.
