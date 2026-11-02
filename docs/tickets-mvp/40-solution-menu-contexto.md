# 40 — Solution: menu de contexto

- [x] `EditorContextMenu` ganhou um nível de submenu e modo `compact` (motivo só no tooltip).
- [x] Menu de projeto: Manage NuGet Packages, Reload Project (re-avaliação do modelo), Build Selected Projects, Run, Debug, Run Unit Test (sempre desabilitado), Edit ▸ (`Edit '<proj>.csproj'`, Find in Files), Copy Path, Properties.
- [x] Menus de raiz, pasta e arquivo: Open, Reload Solution, Copy Path, Find in Files.
- [x] Build/Run/Debug de um projeto usam o gate desse projeto (só console avaliado) e as configurações explícitas (`requestBuild(config)`, `requestRun(config)`, `prepareAndDebug(config)`); o diálogo de buffers sujos preserva o projeto.
- [x] Restricted: itens de execução desabilitados com o motivo existente; "Trust this project" usa o mesmo `grantTrust`.
- [x] Fora (sem backend): Add, Unload, EF Core, Run Multiple, More Run/Debug, Publish, Advanced Build, Local History, Refactor, Inspect, Reformat, Diagrams, Tools, Cut/Copy/Paste/Delete/Rename, Open In, Replace in Files, Edit in Right Split.
