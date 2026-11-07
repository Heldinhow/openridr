# RETRO — Download do netcoredbg limitado por inatividade (issue #162)

Rodada de 2026-11-07, `e52f050` (+ `772d185` da spec), macOS arm64. Spec: [SPEC-DEBUGGER-DOWNLOAD-STALL](SPEC-DEBUGGER-DOWNLOAD-STALL.md). Evidência: [evidence/debugger-download-stall.md](evidence/debugger-download-stall.md). Tickets: #178, #179; mãe #161.

## O que se sustentou

- **A política da #161 era reusável de verdade.** Extrair `http::download_agent(url, stall)` custou ~20 linhas e o download do netcoredbg trocou três timeouts por uma chamada. Nenhum número novo, nenhuma frase nova: `DOWNLOAD_STALL`, `DOWNLOAD_CONNECT`, `stalled`, `is_stall`, `timed_out` — só a visibilidade de dois itens mudou.
- **O laço de redirect próprio não era o problema.** O netcoredbg precisa do allowlist de host oficial por hop; manter o laço e compartilhar o cliente foi o corte certo, e o diff ficou revisável.
- **A falha-antes medida duas vezes, com números.** Com a política antiga (prazo total de 1 s) a transferência de 3 s morre em ~1,25 s com 5.120 de 15.360 bytes; com inatividade chega ao fim. Sem o mapeamento de `TimedOut`, o stall mostra `timed out reading response` em vez da frase do app.
- **A prova longa no host real custou só espera.** 306 s de transferência (6 s além do prazo de 300 s da issue) até `ready` com o progresso subindo, e um stall de 60 s com a frase do app em `debug.error`. Nenhum build de Tauri, nenhum slot de app.
- **O `download_with(stall)` é o mesmo truque de `fetch_with`.** Parametrizar a janela foi o que transformou "o teste passa dos dois jeitos" em guarda de verdade; sem isso o caso lento não distingue a política antiga da nova.

## O que quebrou / custou

- **O download do netcoredbg não é alcançável no app.** Em `tauri dev` o gate de setup só abre com `debug.adapterState == "ready"`, e `require_trust` também exige setup pronto — mas o adaptador é justamente o que o download instala. Medido na ponte de dev: `trust.grant` responde `setup_required`. Com adaptador embutido, `begin_install` recusa. Ou seja: a frase do stall chega a `debug.error` e ao log, mas não há tela alcançável onde ela apareça hoje. É limite do shell de dev (na release o adaptador é embutido e não há download), registrado na evidência sem issue de follow-up, por decisão do orquestrador.
- **A prova de 306 s precisou de um patch local no harness.** O `Route::Slow` do teste de integração foi alterado para 66 pedaços de 5 s, rodado uma vez e revertido; o teste commitado entrega o corpo de uma vez. A evidência diz isso em voz alta, mas o número não é reproduzível só com o repositório — o que dá para reproduzir é a falha-antes de 1 s nos testes de unidade.
- **O primeiro teste lento falhou por margem, não por produto.** 1 KiB a cada 1 s com janela de 1 s morria no stall: o intervalo de envio precisa de folga sobre a janela. Corrigido para 200 ms (5×), a mesma folga que a #161 já tinha escolhido — a lição estava escrita e foi reaprendida.
- **A frase diz "retry the setup" e o botão se chama "Retry adapter install".** Aceito no grill para manter uma frase só; fica como descompasso conhecido em vez de virar duas mensagens.

## Revisão independente (rodada 1) — um alto e um médio, ambos tratados

- **Alto: a URL assinada continuava na mensagem.** Passar `loggable_url` para `describe_error` não bastava: o `ureq` repete a URL dentro do erro, e o `Debug` de `Transport` a traz como `Url { query: Some("X-Amz-Signature=…") }`. `redact_url` remove a URL inteira (forma de texto) e a query (forma de debug) da mensagem final; teste de folha novo com servidor que fecha a conexão e URL assinada, com falha-antes medida (sem `redact_url` o teste reprova com o token no texto).
- **Médio: a prova de 306 s e a de 60 s não eram reproduzíveis.** O patch do harness agora está na evidência como diff, com o comando exato; a spec e o #179 foram alinhados ao caminho que existe de fato (binário de teste do host, não a ponte de dev), com o motivo medido.

## Lições

- **Uma política compartilhada precisa de um ponto de entrada, não de constantes compartilhadas.** `DOWNLOAD_STALL` já era `pub(crate)` e ainda assim o debugger montava o cliente por conta própria com `.timeout(300s)`: o que impede a segunda cópia é a função que monta o cliente, não a constante exportada.
- **Estender a política é mais barato que reescrever o caminho.** O laço de redirect, o allowlist e o hash do netcoredbg ficaram; só o cliente mudou. O diff pequeno é o que permite revisar a parte que importa.
- **"Alcançável pela UI" é uma pergunta de gate, não de código de download.** O caminho existe, está testado e funciona — e ainda assim ninguém o aciona em dev, porque duas guardas (setup e trust) dependem do resultado dele. Vale conferir a alcançabilidade antes de prometer prova de tela.
- **Falha-antes de mensagem se mede desligando o mapeamento.** Trocar `read_failure` por "sempre a frase antiga" e ver o teste reprovar com o texto do transporte é a prova de que o mapeamento é o que muda o que a pessoa lê.

## Follow-ups

- **Circularidade gate × instalação em `tauri dev`**: registrada na evidência; sem issue por decisão do orquestrador (na release não há download).
- **`describe_error` de outros caminhos do host ainda recebe a URL crua**; esta frente corrigiu só a função do netcoredbg. Um passeio de auditoria nas chamadas restantes fica em aberto.
- **Cancelar o download pela UI** e **retomar com Range** continuam fora, como na #161.
