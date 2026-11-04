# 13: Mensagem por projeto e links vivos

**What to build:** Quando falta algo, a UI diz exatamente o que aquele projeto precisa — SDK do projeto (versão pedida, `global.json`/TFM) ou runtime .NET 10 da análise — com link de instalação que funciona e ajuda de instalação.

**Blocked by:** 12 (usa o modelo runtime/SDK separado).

**Status:** ready-for-agent

- [ ] Falta de SDK do projeto cita a versão pedida e a origem (`global.json` ou TFM), sem mandar instalar 10.
- [ ] Falta de runtime 10 diz ".NET 10 runtime", não "SDK 10".
- [ ] O link de instalação abre a página viva (`dotnet.microsoft.com/download/dotnet/10.0` para runtime; página do SDK para o projeto).
- [ ] A ajuda de instalação não exige variável de ambiente nem cópia manual de arquivo.
- [ ] Projeto 8.0.408 sem runtime 10 mostra o pedido certo; com runtime 10 não mostra nada.
- [ ] Textos existentes de "Host SDK 10 is never substituted" são corrigidos onde ainda fizerem sentido.
- [ ] e2e/prova no app real com `OrderDesk` preso ao 8.0.408.
