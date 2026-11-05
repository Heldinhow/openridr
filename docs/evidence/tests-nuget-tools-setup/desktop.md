# Evidência desktop — setup, Tests e NuGet (2026-11-05)

App real: `src-tauri/target/release/bundle/macos/OpenRidr.app` (build desta branch, assinatura ad-hoc),
executado com `OPENRIDR_HOME=/tmp/openridr-desktop-home` e uma cópia de `fixtures/OrderDesk` em
`/tmp/openridr-desktop-work3/OrderDesk` e `/tmp/openridr-desktop-work4/OrderDesk`. Nada foi instalado no sistema; o csharp-ls foi baixado para
o home isolado. Capturas em `docs/evidence/tests-nuget-tools-setup/`. O diretório home foi substituído por `~` nas imagens para não publicar caminho pessoal.

| Captura | O que prova |
|---|---|
| `desktop-setup-first-open.png` | Primeira abertura com home vazio: gate bloqueia o workspace, mostra progresso real (7.538.502 de 21.643.814 bytes), netcoredbg verificado e runtime .NET 10; sem plano/checkbox. |
| `desktop-trust.png` / `desktop-workspace-trusted.png` | Setup concluiu antes do trust; workspace abre em Trusted mode. |
| `desktop-setup-second-open.png` | Segunda abertura no mesmo home: vai direto ao workspace, sem gate e sem novo trust. |
| `desktop-tests.png` | Tests após run real: árvore projeto → namespace → classe → teste preservada, estado agregado (`1 failed`, `4 tests`), barra vertical, contadores, filtro/expand/export e detalhe do teste selecionado com status, duração, projeto e origem. |
| `desktop-tests-run.png` | Run all real anterior, com 4 resultados (2 passed, 1 failed, 1 skipped) e detalhes por resultado. |
| `desktop-nuget-tests.png` | NuGet em `OrderDesk.Tests` com 3 pacotes e 3 updates, seções “Packages to Update”, “Installed Packages” com badge e “Implicitly Installed”, checkbox geral e barra vertical. |
| `desktop-nuget-batch.png` | Update em lote de `xunit` 2.9.2→2.9.3: “Batch update: 1 updated · 0 failed · 1 restore command(s) · restore ok”; o `.csproj` e o `project.assets.json` da cópia foram alterados. |
| `desktop-nuget-solution.png` | Escopo Solution agregando os projetos reais; “Packages to Update in Solution: 2” e “Installed Packages in Solution: 3”. |
| `desktop-nuget-discover.png` / `desktop-nuget-install.png` | Discover real pela fonte configurada e install de `Newtonsoft.Json.Bson` 1.0.3; o `.csproj` da cópia passou a referenciar o pacote e o restore explícito atualizou o assets. |
| `desktop-tools-drawer.png` | Drawer de diagnóstico: netcoredbg embutido com versão, SHA-256 e caminho; sem flag experimental/consentimento; `Reinstall` presente. |

## Limites

- As capturas são da tela real do Mac, mas não provam Windows nem macOS Intel.
- O runtime .NET 10 ausente foi exercitado só no host (`setup_reports_a_missing_bundled_adapter...`
  cobre o adapter; a mensagem de runtime tem teste de unidade), não no app desktop.
- Update em lote e install descoberto foram feitos numa cópia descartável da fixture, com restore real; a fixture do repositório não foi alterada.
- Manifesto por DLL e sessão de solução real continuam fora do escopo/teste; `BaseIntermediateOutputPath` expandido/herdado é reportado como indisponível, não como lista vazia.
- Saída por ⌘Q e por fechar a janela foram provadas com `pgrep -x openridr`, `csharp-ls`, `netcoredbg` e BuildHost vazio; o mesmo vale após `cargo test --workspace`.
- Troca de workspace durante Reinstall (`verifying`, depois do swap): a instalação termina `ready` com uma launcher existente e o `setup.start` seguinte reutiliza; ferramentas são globais, só o shutdown do host as sela.
- O build release foi checado com `nm`/`strings`: sem `allow_setup_for_tests`, sem `processes.probe`/`diagnostics.probe`/`nuget.probe` e sem os nomes de env de probe/teste (`OPENRIDR_ENABLE_PROBE`, `OPENRIDR_TOOLS_MUXER`, `OPENRIDR_BUNDLED_NETCOREDBG`, `OPENRIDR_DEBUG_*`, `OPENRIDR_FORCE_HOSTROOT`).
- `BaseIntermediateOutputPath` expandido/herdado é reportado como “implícitos indisponíveis (obj fora do padrão)”; a mensagem tem teste de host e não é uma lista vazia.
