# Instalação offline do csharp-ls

- O pacote 0.28.0 foi baixado da flat container oficial; SHA-512 calculado corresponde ao `packageHash` do catálogo, registrado em `docs/decisions.md`.
- Testes unitários `package_hash_must_match_exactly` e `persisted_installation_detects_launcher_changes` passam; o primeiro também verifica que bytes incorretos não produzem uma fonte local.
- `cargo test -p openridr-host --test host tools_install_is_real_verified_and_reused -- --test-threads=1 --nocapture` passou usando .NET real e home temporário: instalou o pacote conferido via fonte local e reutilizou a instalação em um novo host sem iniciar outro instalador.
- `tools_download_does_not_block_host_calls` falhava com download simulado de 1,5 s; passa com download em worker, e prova que fechar o workspace antes de terminar não inicia o instalador. A transferência registra bytes observados e tamanho declarado quando disponível.
- Verificado no app desktop em 2026-11-05: gate com progresso real, erro/retry e segunda abertura sem gate; ver `desktop.md`.
