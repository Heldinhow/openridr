# 07 — b13: MissingSdk diz o que fazer

Status: ready-for-agent
Achado: QA-BUGHUNT-2026-11-02 §13.

## Aceite

- Com `projectSdk.state === "sdk_missing"`, o motivo de avaliação desabilitada nomeia a versão pinada e diz que o host não instala nem substitui SDK.
- A área oferece: `Redetect SDKs` (chama `sdk.redetect`), link oficial (`instructionsUrl`, nova aba) e `Open global.json`.
- Nenhum processo de instalação ou escrita em `global.json` é disparado; o link só abre o navegador.
- O estado de trust não é alterado.

## Teste

E2E em `fixtures/MissingSdk`: mensagem contém `8.0.999`, link aponta para o domínio oficial, botão redetect existe e `sdk.project.state` continua `sdk_missing` depois do clique.
