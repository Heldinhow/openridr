# 15: Proposta de embutir o netcoredbg (só documento)

**What to build:** Uma proposta curta e medida para embutir o netcoredbg no app, sem implementar nada nesta rodada. O SDK .NET nunca é embutido.

**Blocked by:** 06 (o comportamento e o tamanho do instalador precisam estar estabilizados).

**Status:** proposta concluída — decisão pendente de Helder.

- [x] Tamanho medido do artefato oficial osx-arm64 3.2.0-1092 (3.39 MB) e impacto no DMG.
- [x] Custo de assinatura/notarização do binário embutido e de mantê-lo atualizado.
- [x] Alternativas comparadas: download no primeiro Debug (atual), embutir, instalar sob demanda com bundle de ferramentas.
- [x] Recomendação explícita e o que ficaria fora (SDK .NET jamais embutido).
- [x] Nenhum código de embed; nenhuma mudança de comportamento nesta rodada.
