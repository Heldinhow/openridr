# 29 — Fontes configuradas do NuGet

- [x] Usar `dotnet nuget list source --format short`, formato suportado pelo SDK 8, sem alterar `global.json`.
- [x] Aceitar fontes habilitadas e desabilitadas, caminhos com espaços e lista vazia; saída desconhecida é erro, não lista vazia.
- [x] Identificar na UI as fontes configuradas e avisar que a origem por pacote não é reportada.
- [x] Testar parser de host (13/13), probe de UI (4/4) e consulta real em cópia temporária de OrderDesk (3 pacotes, fonte habilitada, SDK 8.0.408).
- [x] `bun run check`; suíte completa 101/101 anterior à mudança, com testes dirigidos posteriores.

A consulta real e as limitações estão em `docs/evidence/28-rider-visual-pass.md`. Windows e macOS Intel não foram executados.
