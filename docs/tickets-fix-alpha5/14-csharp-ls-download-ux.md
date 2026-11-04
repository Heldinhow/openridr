# 14: Download do csharp-ls com progresso, retry e erro claro

**What to build:** Na primeira abertura de um projeto C#, o download do csharp-ls (~99 MB) mostra progresso, permite tentar de novo e falha com mensagem clara quando a rede cai. O plano revisável continua sendo a única via de instalação.

**Blocked by:** 06 (reutiliza o mesmo componente de download: progresso, retry, sha256 e registro visível) e 12 (runtime/SDK separados).

**Status:** ready-for-agent

- [ ] A primeira abertura de projeto C# sem análise instalada expõe o plano de instalação (mantendo a revisão explícita).
- [ ] Confirmado o plano, o download mostra progresso observado.
- [ ] Falha de rede mostra erro acionável e botão de tentar de novo, sem exigir reabrir o workspace.
- [ ] Instalação parcial nunca é marcada como pronta; retry recomeça do staging limpo.
- [ ] Instalação verificada existente é reutilizada após confirmação, sem baixar de novo.
- [ ] O fluxo não muda para quem já tem o csharp-ls instalado.
- [ ] Teste de host com falha de rede simulada e prova no app real de progresso/erro/retry.
