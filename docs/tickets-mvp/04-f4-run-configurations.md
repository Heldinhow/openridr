# 04 — F4: Run configurations

**What to build:** configurações de execução salvas por solução, com diálogo de edição, configuração ativa como fonte da verdade de projeto/Configuration/TFM/argumentos/cwd/env/.env e Run/Debug no topo usando os gates existentes.

**Blocked by:** 01

**Status:** done — commit `feat(F4): add run configurations wired to build, run and debug`; `cargo test -p openridr-host` e os specs de build/run/debugger/evaluation/run-configs/aceitação verdes.

- [x] Existe uma configuração padrão "Run <StartupProject>" derivada da avaliação, somente leitura; editar cria uma cópia nomeada.
- [x] O diálogo permite criar, editar, duplicar e remover configuração com nome, projeto, Configuration, TFM, argumentos, diretório de trabalho, variáveis e arquivos `.env`; valida campo e explica o erro antes de executar.
- [x] A configuração ativa aparece na barra e é a fonte da verdade; mudar Configuration/TFM na barra atualiza essa configuração e dispara nova avaliação.
- [x] Run e Debug executam a configuração ativa pelos fluxos existentes (`build.start` + `run.start`/`debug.start`), com os gates de restore, plataforma e revisão; nenhum pipeline novo.
- [x] Trocar de configuração nunca reutiliza TargetPath de outra; configuração inválida é recusada com motivo e ação.
- [x] Antes de build/run com buffers sujos, a UI oferece "Save all and build", "Build saved content (N unsaved buffers excluded)" e "Cancel"; nunca salva sozinha.
- [x] Configurações e id ativo persistem por solução no app home e sobrevivem a reload; nada é gravado dentro do repositório.
- [x] Arquivos `.env` aceitam `KEY=VALUE`, comentário com `#`, sem expansão nem shell; linha inválida é reportada no diálogo.
- [x] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam; commit local `feat(F4): ...`.
