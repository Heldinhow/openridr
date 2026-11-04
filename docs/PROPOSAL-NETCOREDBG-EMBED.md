# PROPOSTA — Embutir o netcoredbg no app

**Status:** aprovada e implementada em 2026-11-04 (`f804cd8`). Spec: `docs/SPEC-EMBED-NETCOREDBG.md`; evidência: `docs/evidence/embed-netcoredbg.md`. As decisões pendentes no fim deste documento foram respondidas pelo Helder.
**Data:** 2026-11-04. **Alvo:** macOS Apple Silicon (o único alvo distribuído hoje).

## Medição

Medido nesta máquina em 2026-11-04, a partir do asset oficial e de uma instalação real do
`3.2.0-1092`:

| Item | Bytes | Equivalente |
|---|---:|---:|
| `netcoredbg-osx-arm64.zip` oficial (3.2.0-1092) | 3.389.504 | 3,23 MiB / 3,39 MB |
| Árvore extraída (7 arquivos, inclui Roslyn e `libdbgshim.dylib`) | 9.587.448 | 9,14 MiB / 9,59 MB |
| Rezip da árvore extraída (mesma família de compressão do DMG) | 3.388.944 | 3,23 MiB |
| DMG atual da alpha.5 | 5.604.275 | 5,34 MiB / 5,60 MB |
| App instalado hoje (`du -sh`) | ~14 MB | passaria a ~23 MB |

**Impacto estimado no DMG:** +3,4 MB comprimidos, de 5,34 MiB para **~8,6 MiB** (5,60 → ~8,99 MB).
O rezip da árvore deu 3.388.944 bytes, praticamente o tamanho do zip oficial (3.389.504), então a
estimativa de compressão é confiável. O número final deve ser medido no DMG assinado e notarizado;
assinatura e notarização adicionam alguns KB, não MB.

SHA-256 do zip oficial: `f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea`.

## O que muda ao embutir

- **Rede:** o download do netcoredbg no primeiro Debug desaparece. O app **não** fica offline-first:
  a análise C# ainda baixa o csharp-ls (~99 MB) e o Debug ainda exige o SDK .NET e um build do
  usuário. Embutir resolve só a dependência de rede do adaptador.
- **Versão:** o app passa a fixar o netcoredbg que ele próprio distribui. Trocar de versão exige
  release do app; não há atualização independente da ferramenta.
- **Assinatura:** o binário oficial é **ad-hoc/linker-signed** (`codesign -dv`: `flags=adhoc,
  linker-signed`). Para o DMG notarizado, o executável e a `libdbgshim.dylib` embutidos precisam ser
  reassinados com Developer ID + hardened runtime e entrar na notarização. O app hoje não usa
  entitlements; se o runtime do .NET exigir JIT/executable memory para o adaptador, isso vira uma
  exceção de entitlements a aprovar (não verificado nesta proposta).
- **Atualização:** correção ou melhoria do upstream só chega com uma nova release do app. O pin
  atual também é manual, mas hoje dá para trocar a versão sem reassinar o app inteiro.
- **Licença/notice:** netcoredbg é MIT (Copyright © 2017 Samsung Electronics Co., LTD). O zip
  oficial não traz arquivo `LICENSE`, então o app precisa distribuir o texto MIT e os notices das
  DLLs embutidas (Roslyn/dbgshim) em um `THIRD-PARTY-NOTICES` — item novo de release.
- **Tamanho:** +9,14 MiB no app instalado e ~+3,4 MB no DMG, para todo usuário, inclusive quem nunca
  usa Debug.
- **Instalação:** o caminho de download/staging/progresso/retry/checksum do netcoredbg deixa de ser
  necessário. O host continua verificando `--version` e pode conferir o hash do arquivo no build.
  Remover esse código é trabalho futuro; nesta rodada nada muda.

## Alternativas

| Opção | A favor | Contra |
|---|---|---|
| **Download no primeiro Debug (atual)** | DMG menor; só quem depura paga o custo; ferramenta atualizável sem release do app; fluxo já testado (progresso, SHA-256, retry) | Primeiro Debug depende de rede e da disponibilidade do GitHub; mais partes móveis; falha offline sem adaptador |
| **Embutir no bundle (recomendada)** | Debug funciona sem rede depois de instalado; versão e hash determinísticos; um único artefato assinado/notarizado; menos código de instalação no futuro | +3,4 MB no DMG e +9,14 MiB instalado; reassinatura/notarização do binário aninhado; atualização do adaptador acoplada à release do app; notice MIT; todo usuário paga o tamanho |
| **Bundle de ferramentas sob demanda** (zip assinado do próprio OpenRidr, baixado na primeira necessidade) | Mantém o app pequeno; permite assinar/notarizar as ferramentas uma vez; tira a dependência do GitHub | Continua exigindo rede e uma segunda versão para gerenciar; precisa de hospedagem/canal próprio; não elimina o primeiro uso sem rede |

O bundle sob demanda não compensa para um único binário de 3,4 MB: cria um canal de distribuição
novo para economizar pouco espaço. O csharp-ls não cabe nesse bundle sem carregar o runtime .NET
junto, o que está fora de escopo.

## Recomendação

**Embutir o `netcoredbg-osx-arm64` 3.2.0-1092 no bundle**, assinado com Developer ID e coberto pela
notarização do app, e **aposentar o download do netcoredbg** na mesma mudança (sem fallback de rede:
o bundle assinado é a fonte da verdade). Manter a verificação `--version` do host e registrar no
`THIRD-PARTY-NOTICES` a licença MIT e os componentes embutidos.

Embutir só o osx-arm64; Windows e macOS Intel continuam fora do que é distribuído e testado. O
primeiro passo depois da aprovação é medir o DMG assinado real e provar que o binário embutido roda
a partir de `Contents/Resources` com o app montado e com quarentena.

## Riscos explícitos

1. **Notarização:** binário e dylib aninhados ad-hoc são motivo de rejeição; é preciso reassinar e
   renotarizar, e talvez adicionar entitlements de JIT. Sem prova em DMG real, o custo é desconhecido.
2. **Execução em bundle read-only:** hoje o adaptador roda em `<app home>/tools/...` gravável. Rodar
   de `Contents/Resources` (somente leitura, com quarentena/translocation) não foi testado; se o
   netcoredbg escrever ao lado do executável, isso quebra.
3. **Latência de atualização:** um bug do adaptador só é corrigido em release do app, não por update
   da ferramenta.
4. **Tamanho permanente:** +9,14 MiB instalados e ~+3,4 MB no DMG para todos.
5. **Licença:** o zip não inclui `LICENSE`; publicar sem o notice MIT é violação, e os notices das
   DLLs embutidas precisam ser conferidos.
6. **Suporte upstream:** o release continua "community supported", com DAP sem evidência oficial
   (ticket 14); embutir não melhora esse status.

## Fora de escopo

- **SDK .NET nunca é embutido.** O app só detecta SDKs; não instala nem edita `global.json`.
- **csharp-ls continua baixado pelo usuário**, com o fluxo revisável do ticket 14 intacto (e o
  runtime .NET 10 que ele exige).
- Windows, macOS Intel, Linux e quaisquer outros assets do netcoredbg.
- Auto-update independente de ferramentas ou um serviço de distribuição de tool bundle.
- Qualquer código, mudança de comportamento ou dependência nova nesta rodada.

## Decisão pendente de Helder

1. Embutir ou manter o download no primeiro Debug?
2. Se embutir: aceitar DMG ~8,6 MiB / app ~23 MB e atualização do adaptador somente com release do
   app?
3. Se embutir: aprovar reassinatura/notarização do binário aninhado e os entitlements que a prova
   exigir, além do `THIRD-PARTY-NOTICES`.
