# 24 — Auditoria visual: NuGet sem pacotes e tabela

**What to build:** Representar corretamente projeto sem PackageReference e tornar a lista/fonte de pacotes útil na área padrão.

**Source:** `docs/UI-AUDIT.md` A8/M9; F9 #70–75 e decisão F9 em `docs/decisions.md`.

**Blocked by:** nenhum.

**Status:** done

- [x] `dotnet list package --outdated` sem array `frameworks` por ausência de pacotes resulta em estado explícito “nenhum pacote” (ou equivalente), não erro bruto nem falso sucesso de atualização.
- [x] Lista de pacotes começa visível na altura padrão do painel e suas colunas/valores são legíveis; overflow usa rolagem apropriada.
- [x] Falha de `dotnet nuget list source` não aparece como erro vermelho se consulta de pacotes funcionou; mostrar separadamente fonte desconhecida/consulta parcial com estado honesto.
- [x] Multi-TFM, CPM, update confirmado e restore explícito preservam contratos; adicionar testes de parsing para saída que omite `frameworks`.
- [x] E2E cobre fixture sem pacote e com pacote, sem update/restores implícitos; preservar testids. O E2E usa probe determinístico, sem executar dotnet.
- [x] Executar build, testes parser/boundary e E2E aplicável; registrar quando SDK/rede reais não estiverem disponíveis. Não foi executado NuGet real nesta fatia.

## Comments

- Capturas: `60-nuget-no-packages.png`, `59-nuget-packages.png`.
- Mudança de parser é necessária, não apenas CSS: ausência de frameworks é saída válida do CLI quando não há pacotes.
