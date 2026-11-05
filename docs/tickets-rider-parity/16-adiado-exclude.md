# 16 — Adiado: Exclude (G16)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Out of Scope". **Bloqueado por:** nenhum para
documentar; a implementação depende de backend novo. **Status:** adiado.

## O que falta

Excluir um arquivo do projeto sem apagá-lo do disco, editando o `.csproj` (remove/update do
item) e reavaliando. Hoje o host não escreve no `.csproj`.

## Estado atual (não muda nesta onda)

- Arquivo e pasta têm Renomear e Excluir (do disco); não há "Exclude from Project" falso.
- Nenhum item de menu promete a operação sem backend.

## O que destrava

1. Caminho de host para remover um item do `.csproj` com validação e rollback.
2. Reavaliação/restore e atualização da árvore depois da exclusão.
3. Teste com fixture: excluir um arquivo do projeto, conferir que ele continua no disco e que
   o build não o compila.

## Aceite (deste ticket de registro)

- [ ] O menu não exibe "Exclude from Project" sem backend.
- [ ] O adiamento, o motivo e o que falta estão registrados.
