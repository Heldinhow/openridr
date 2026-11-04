# Log bruto e2e alpha.5 (append-only)
Ambiente: OpenRidr 0.3.0-alpha.5 (cópia em ~/Applications-test/OpenRidr-alpha5), branch test/e2e-alpha5 @ 5c83aae, macOS 27.2 arm64.
Web: web.sh com VERIFY_OPENRIDR_STATE=/tmp/alpha5-web (UI 1520, host 4417).
[EXCEÇÃO web] Usei a superfície web (web.sh + browser) só para: abrir .sln, Trust, árvore, abrir Program.cs. Regra reforçada pelo usuário: teste pelo desktop real. Parei aqui.
[RECEITA] agent-desktop type sem --headed no CodeMirror: 'delivered_unverified' e o texto NÃO entra (ACTION_FAILED post_action_verification). Com --headed: 'delivered_verified'.
[RECEITA] press sem --headed: 'Application has no verified focused element for keyboard delivery'. Com --headed funciona (focus-window antes).
[RECEITA] Atalhos headed: 'press cmd+z' e 'press cmd+s' funcionam após focus-window + focus do editor. Cmd+S gravou no disco. Cmd+Z reverteu a digitação e a aba voltou a limpa.
[OK] Save pelo botão 'Save' grava no disco; aba sai de 'modified'.
[ACHADO] Árvore não reflete create/rename/delete externo (20s) — só após Re-evaluate.
[OK] Save com disco alterado externamente: diálogo 'File changed on disk' (Reload / Overwrite). Disco preservado.
[OK] Save de buffer cujo arquivo foi apagado por fora: 'Save failed; buffer kept. File not found' (não recria).
[P3?] Aba aberta de arquivo apagado externamente fica limpa sem aviso até o Save.
[WRAPPER] act.py: 'Close Order.cs' casou no lugar de 'Order.cs' (substring). Usar '=Nome' para match exato.
[ACHADO P2?] Executar o binário direto com o app já aberto abre 2ª instância (PID 36017, mesmo OPENRIDR_HOME); não há repasse à instância existente. Caminho real (open -a / Finder) ainda não testado.
[SETUP] launchctl setenv OPENRIDR_HOME=/tmp/alpha5-desktop/home para o caminho LaunchServices; será desfeito com unsetenv.
[NOTA] scripts/openridr sem OPENRIDR_APP abre 'OpenRidr' = /Applications (alpha.4 instalado).
[SKILL-ERRO] features/open-workspace/open-with dizem 'com workspace aberto -> diálogo de troca (externalOpen)'. No código (WorkspaceView.tsx:3816) não há diálogo de troca: openSolution direto. Observado: Trust prompt do csproj; Cancel mantém o .sln anterior intacto.
[BUG-cand P2] Buffer de aba LIMPA não recarrega quando o arquivo muda no disco (20s, aba ativa, conteúdo antigo). Recarrega só ao trocar de aba/focar. Sem aviso.
[OK] Problems: CS1003/CS1002 com linha:coluna corretos; clicar no erro posiciona Ln 3, Col 18.
[OK] Build succeeded exit 0; Run 'Order A-104: R$ 42,00' exit 0; Discover 4 tests; Run all -> 2 passed, 1 failed (fixture deliberada), 1 skipped.
[BUG cand P1] Apagar arquivo .cs por fora sem Re-evaluate: árvore continua listando o arquivo; Build ok; Debug fica bloqueado com "saved source changed after this build (src/OrderDesk.Core/Renamed.cs). Rebuild before debugging." mesmo após rebuild. Só Re-evaluate desbloqueia. Mensagem cita arquivo inexistente.
[SKILL-ERRO] build-test-debug.md diz que existe diálogo debug-consent-dialog. Código (useExecutionFlow.ts:291) diz 'One-click Debug: there is no consent question' e instala netcoredbg/csharp-ls no clique. Observado: sem diálogo; netcoredbg 3.2.0-1092 e csharp-ls 0.28 baixados em tools/ às 21:51. Revisar decisão (segurança).
[ACHADO P2] Argumento .cs ou .txt: abre Welcome sem nenhuma mensagem explicando (silencioso).
[ACHADO P3] c6 sln inválido: após Trust mostra "Evaluation needs a resolved project SDK" (mensagem não descreve sln inválido).
[ACHADO P3] c8 TFM inválido (net99.0): 'Evaluated with errors · stale · revision 1' e 'Values shown are from the previous evaluation' na primeira avaliação (não existe anterior).
[BUG cand P0] Cmd+Q (via press cmd+q --force, AXPress 'Quit' menu) com buffer sujo ('QQ' em Order.cs): app encerrou SEM diálogo; QQ perdido (não está no disco nem é restaurado ao reabrir).
[BUG cand P0] Quit por 'quit' AppleScript com buffer sujo ('RR1'): sai sem diálogo; RR1 perdido.
[OBS] Botão Close da janela: processo sai ~segundos depois; verificar CL7 (ver próxima nota).
[RECURSOS] idle (Order.cs aberto, 1 min): RSS ~96MB, CPU 0.3%, 0 DiagnosticReports com openridr.
[CONFIRMADO P0] Cmd+Q (press cmd+q --window-id, sem diálogo) numa janela com buffer sujo 'QZ9': app saiu sem perguntar; QZ9 não foi ao disco. Reproduzido também via Cmd+Q(menu) e 'quit' AppleScript.
[CONTRASTE OK] Fechar janela (botão Close) com buffer sujo: diálogo 'Close OpenRidr with unsaved changes?' com Save all / Keep editing / Discard and close app. 'Save all' gravou o CL7 no disco.
- seletor '=Close' pegou a janela principal (fechou o app sem diálogo; buffer limpo). Erro do wrapper, não do app.
[P2] Arquivo .cs em UTF-16 (BOM) não abre: 'Could not read src/.../Utf16.cs as UTF-8 text' (mensagem clara; suporte a UTF-16 ausente).
[OK] Binário com extensão .cs: 'Could not read ... as UTF-8 text'.
[BUG P2] Save de arquivo CRLF converte para LF (0 CR após salvar; file: ASCII text). Diff inteiro no git.
[OK] Arquivo CRLF abre; Huge.cs (28MB) abre no editor (RSS ~180MB).
[LACUNA] Árvore sem criar/renomear/apagar arquivo na UI (menus do projeto e 'Options' só têm ações de projeto/avaliação). Só alterações externas.
[P3] Menu de contexto de projeto se chama 'Solution context menu'.
[NAO ALCANCADO] Close Workspace: disclosure 'Project: OrderDesk.sln' sem ref na árvore; não consegui fechar o workspace por agent-desktop.
[SIGKILL durante build] build pode ter terminado antes; sem órfãos (dotnet/csharp-ls/BuildHost) com caminho alpha5 após kill -9. Limitado: build rápido.
[ACHADO P2?] Plano do netcoredbg diz 'stays blocked on this platform' mas o binário já está instalado (baixado no clique de Debug). Inconsistência de estado/UI.
[PARCIAL] Install analysis: fluxo tem plano + 'Confirm and install' explícito (bom). Falha de rede NÃO concluída: o clique de confirmação caiu no plano do netcoredbg (dois 'Review install plan'/'Confirm and install' sem rótulo distinto). Dir tools/csharp-ls criado vazio às 22:12.
[SKILL BUG P2] desktop.sh down: imprime 'stopped' mas a instância real (50011, iniciada à mão) continua viva; app.pid estava com PID antigo (36017). Limpeza da skill (pgrep -x openridr vazio) falharia.
[SKILL] crates/host/tests/trust_model.rs existe; referências a tests/e2e/*.spec.ts e docs existem (checado estaticamente; specs NÃO rodados).
