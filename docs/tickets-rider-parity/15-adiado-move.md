# 15 — Adiado: Move (G16)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Out of Scope". **Bloqueado por:** nenhum para
documentar; a implementação depende de backend novo. **Status:** adiado.

## O que falta

Mover arquivo ou pasta dentro do workspace atualizando o `.csproj` (includes, links e caminhos
relativos), as abas abertas e as referências. Hoje o host só cria, renomeia e exclui.

## Estado atual (não muda nesta onda)

- Arquivo e pasta têm Renomear e Excluir; não há "Move" falso.
- Nenhuma operação de mover é executada por caminho não suportado.

## O que destrava

1. Operação de host para mover com atualização do projeto e rollback em falha.
2. Reavaliação/restore depois do move e atualização das abas abertas.
3. Teste com fixture: mover um arquivo incluído, buildar e conferir o `.csproj`.

## Aceite (deste ticket de registro)

- [ ] O menu não exibe "Move" sem backend.
- [ ] O adiamento, o motivo e o que falta estão registrados.
