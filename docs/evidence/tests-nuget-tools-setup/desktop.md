# Evidência desktop — setup, Tests e NuGet (2026-11-05)

App real: `src-tauri/target/release/bundle/macos/OpenRidr.app` (build desta branch, assinatura ad-hoc),
executado com `OPENRIDR_HOME=/tmp/openridr-desktop-home` e uma cópia de `fixtures/OrderDesk` em
`/tmp/openridr-desktop-work/OrderDesk`. Nada foi instalado no sistema; o csharp-ls foi baixado para
o home isolado. Capturas em `docs/evidence/tests-nuget-tools-setup/`. O diretório home foi substituído por `~` nas imagens para não publicar caminho pessoal.

| Captura | O que prova |
|---|---|
| `desktop-setup-first-open.png` | Primeira abertura com home vazio: gate bloqueia o workspace, mostra progresso real (7.538.502 de 21.643.814 bytes), netcoredbg verificado e runtime .NET 10; sem plano/checkbox. |
| `desktop-trust.png` / `desktop-workspace-trusted.png` | Setup concluiu antes do trust; workspace abre em Trusted mode. |
| `desktop-setup-second-open.png` | Segunda abertura no mesmo home: vai direto ao workspace, sem gate e sem novo trust. |
| `desktop-tests.png` | Tests com abas, contadores, expand/collapse, export, árvore e painel de detalhe; descoberta automática lista 4 testes. |
| `desktop-tests-run.png` | Run all real: 4 resultados, 2 passed, 1 failed, 1 skipped, duração e detalhes por resultado. |
| `desktop-nuget.png` / `desktop-nuget-tests.png` | NuGet com abas, escopo, fonte, prerelease, Installed/Discover, seção de updates e implícitos/transitivos; `OrderDesk.Tests` com 3 pacotes e 3 desatualizados. |
| `desktop-nuget-discover.png` | Discover real: 30 resultados de `https://api.nuget.org/v3/index.json` para “Newtonsoft”, com versão, autores e ação Install. |
| `desktop-tools-drawer.png` | Drawer de diagnóstico: netcoredbg embutido com versão, SHA-256 e caminho; sem flag experimental/consentimento; `Reinstall` presente. |

## Limites

- As capturas são da tela real do Mac, mas não provam Windows nem macOS Intel.
- O runtime .NET 10 ausente foi exercitado só no host (`setup_reports_a_missing_bundled_adapter...`
  cobre o adapter; a mensagem de runtime tem teste de unidade), não no app desktop.
- Install de pacote descoberto e update em lote não foram confirmados contra a fixture real; os
  planos e as recusas estão cobertos por testes de host. A fixture não foi alterada.
- Escopo Solução não foi aberto no app real; a agregação tem contrato no host, sem captura.
