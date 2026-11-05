# Retro — setup de ferramentas, Tests e NuGet (2026-11-05)

## O que mudou

- Ferramentas fixas passaram a ser setup obrigatório: `setup.start` sem trust, gate antes do workspace, progresso/erro/retry, reutilização entre projetos e abertura direta na segunda vez. O drawer virou diagnóstico.
- Tests ganhou abas, contadores, estado agregado, detalhe da seleção, expandir/recolher e exportação JSON real; debug de teste, Continuous Testing e cobertura ficaram desabilitados com motivo.
- NuGet ganhou escopo Solução, fonte, prerelease, implícitos/transitivos de `project.assets.json`, updates em lote e Discover v3 pela fonte escolhida.
- Evidência desktop em `evidence/tests-nuget-tools-setup/desktop.md`, com capturas antes/depois e app real.

## Lições

- **Setup obrigatório muda a infraestrutura de teste.** O home compartilhado do E2E passou a precisar de um csharp-ls *real*; semear um launcher falso quebrou todos os specs de LSP com "Failed" em vez de "Ready". O seed agora baixa o pacote pinado e roda `dotnet tool install` na primeira execução, e copia a instalação real para os homes isolados.
- **O gate não pode ser testado esperando o texto "Ready".** Quando o setup termina, o gate desmonta e o trust aparece; a asserção precisa esperar o trust/o workspace, não um estado transitório que o usuário nunca vê.
- **Testes de install antigos codificavam o plano/confirm.** Foram substituídos pelo fluxo `setup.start`/retry; manter os specs antigos só produziria falhas de testid e falsa cobertura.
- **A instalação assíncrona mudou o cancelamento.** Cancelar durante o download pode não ter processo ainda; a nota/log precisa continuar dizendo "not ready", e o teste de terminação precisa esperar o processo controlado. Launcher antigo sem integridade é verificado e adotado, senão um upgrade baixaria o mesmo pacote de novo.
- **NuGet real é lento e não é livre de rede.** Os testes de UI continuam no probe; o desktop usou rede real para listar/descobrir. Updates em lote/install descoberto não foram confirmados contra a fixture para não alterá-la; ficam como pendência explícita.
- **Captura de tela do gate exige corrida.** O download real leva poucos segundos; a evidência foi capturada no meio do download (7,5 MB de 21,6 MB). Segunda abertura foi capturada depois de fechar e reabrir o app com o mesmo home.
- **Texto inline em `<strong>` + `<span>` colou** ("csharp-ls 0.28.0Downloading…"); foi corrigido com grid/`display:block` no CSS do gate antes da captura final.

## Pendências

- Windows, macOS Intel e runtime .NET 10 ausente no app real.
- Confirmação real de update em lote e de install descoberto; escopo Solução no app real.
- Debug de teste, Continuous Testing, cobertura e sessão "All tests from Solution" continuam fora do host.
