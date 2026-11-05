# 14 — Adiado: Add Existing Item (G16)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Out of Scope". **Bloqueado por:** nenhum para
documentar; a implementação depende de backend novo. **Status:** adiado.

## O que falta

Adicionar à pasta/projeto um arquivo que já existe no disco, com diálogo nativo de seleção e
atualização do `.csproj`. Hoje não há seletor de arquivos nativo (nenhuma dependência de
diálogo) nem escrita de itens do projeto no host.

## Estado atual (não muda nesta onda)

- A pasta já tem Novo arquivo, Renomear e Excluir; não há "Add Existing Item" falso.
- Nenhum item de menu é exibido sem backend.

## O que destrava

1. Um seletor de arquivos nativo (Tauri dialog ou equivalente) aprovado como dependência.
2. Um caminho de host para incluir/remover itens do `.csproj` com validação e rollback.
3. Teste com fixture: adicionar um arquivo existente, reavaliar e buildar.

## Aceite (deste ticket de registro)

- [ ] O menu de pasta/projeto não exibe "Add Existing Item" sem backend.
- [ ] O adiamento, o motivo e o que falta estão registrados.
