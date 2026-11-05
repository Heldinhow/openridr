# Instalação offline do csharp-ls

- O pacote 0.28.0 foi baixado da flat container oficial; SHA-512 calculado corresponde ao `packageHash` do catálogo, registrado em `docs/decisions.md`.
- Testes unitários `package_hash_must_match_exactly` e `persisted_installation_detects_launcher_changes` passam; o primeiro também verifica que bytes incorretos não produzem uma fonte local.
- `cargo test -p openridr-host --test host tools_install_is_real_verified_and_reused -- --test-threads=1 --nocapture` passou usando .NET real e home temporário: instalou o pacote conferido via fonte local e reutilizou a instalação em um novo host sem iniciar outro instalador.
- Ainda não verificado: gate de setup, progresso durante o download, retry visual e comportamento do desktop. O download atual ocorre antes da criação do processo instalador e precisa ser integrado ao estado de setup sem bloquear a interface.
