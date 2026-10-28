# 13: Instalar debugger com confirmação e gates de plataforma

**What to build:** Planejar, confirmar e verificar netcoredbg 3.2.0 de asset oficial Samsung compatível, deixando claras indisponibilidade por arquitetura e flag experimental ARM64 independente de instalação/trust. Verificação do binário ainda não prova DAP.

**Blocked by:** 02 — Confiar no workspace e separar SDKs.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Plano informa versão aprovada/release 3.2.0-1092, URL/asset oficial Samsung, SO/arquitetura, destino absoluto fora do repo, operação e permissões. Sem asset compatível verificado, origem válida ou consentimento, não baixar/instalar; não substituir versão silenciosamente.
- [x] Aplicar integralmente G3: validar origem e redirects, destino e extração segura sem escape, mecanismos de integridade/autenticidade disponíveis com limites explícitos. Bloquear divergência/corrupção e uso não verificado; sem global install/elevação implícita/SDK installer; plano alterado exige reconfirmação.
- [x] Exercitar plano pendente/downloading/installing/verifying/ready/cancelled/failed, logs duráveis, cancelamento, retry e parcial não pronta, preservando ferramenta válida. Progresso só medido; sucesso verifica executável/versão, não declara DAP funcional.
- [x] Mac ARM64 inicia `Apple Silicon debugger: experimental · disabled`, flag OFF, Debug bloqueado e Run independente. Opt-in separado de trust e consentimento não instala nem comprova compatibilidade; flag OFF recusa instalação (`experimental_required`), flag ON + trust + consentimento instalam o asset oficial osx-arm64; só 14/18 podem reunir prova para futura retirada da flag.
- [ ] Mac Intel sem asset 3.2 compatível verificado mostra indisponível e bloqueia instalação/launch, sem baixar outra versão. Windows x64 requer asset adequado e posteriormente smoke DAP próprio. Limites não desabilitam análise/Run/testes disponíveis. (Gate/plano e ausência de asset provados em teste de host e no E2E desta máquina; nenhuma máquina Intel ou Windows foi executada.)
- [x] Host aplica consentimento/plataforma/flag/trust quando pertinentes, inclusive chamadas diretas. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), incluindo auditoria MIT/avisos e UI acessível. Este ticket independe do instalador csharp-ls.

## Mapeamento à SPEC

AC-04/05/06 (parcial: instalador debugger completo; csharp-ls em 05), AC-22 (parcial: adaptador ausente bloqueia), AC-23 (flag/opt-in), AC-24 (assets/limites; smoke em 14), AC-01/07 e AC-32/33/34 na fatia. SPEC P0.1/P0.5, Avaliação e ferramentas e Licenças.

## Validação

UI real Mac para plano/recusa/flag/estados e instalação oficial somente onde asset compatível existe; ausência legítima exige prova de bloqueio, não download alternativo. Boundary determinística: redirect/origem inválidos, arquitetura errada, path traversal/extração, corrupção/verificação, plano trocado, cancelamento/falha/retry e permissões. Smoke de binário real não substitui DAP; registrar versões/assets/logs e pendências Windows/Intel explicitamente.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e evidenciado em macOS 27.2 arm64; revisão no mesmo dia após checar o gate. A release oficial 3.2.0-1092 publica `netcoredbg-osx-arm64.zip` (sha256 `f4fa33…4e59ea`), declarado *community supported* pelo README da tag. Flag OFF recusa instalação com `experimental_required`; flag ON + trust + consentimento instalaram o asset oficial no app home real (1,6 s, sha256 conferido, `NET Core debugger 3.2.0-1 (9744e1f, Release)`, executável sha256 `ffc51b6f…24ece`) e um novo plano reutilizou a instalação sem baixar. Plano/consentimento/origem/redirect/extração/hash/estados/retry/cancel também foram exercitados com servidor local e zip controlado. Faltam: launch/DAP real (ticket 14), instalação/smoke em Windows x64 e Mac Intel — ver [evidência](../evidence/13-instalacao-debugger.md).
