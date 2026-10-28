# 13: Instalar debugger com confirmação e gates de plataforma

**What to build:** Planejar, confirmar e verificar netcoredbg 3.2.0 de asset oficial Samsung compatível, deixando claras indisponibilidade por arquitetura e flag experimental ARM64 independente de instalação/trust. Verificação do binário ainda não prova DAP.

**Blocked by:** 02 — Confiar no workspace e separar SDKs.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Plano informa versão aprovada/release 3.2.0-1092, URL/asset oficial Samsung, SO/arquitetura, destino absoluto fora do repo, operação e permissões. Sem asset compatível verificado, origem válida ou consentimento, não baixar/instalar; não substituir versão silenciosamente.
- [ ] Aplicar integralmente G3: validar origem e redirects, destino e extração segura sem escape, mecanismos de integridade/autenticidade disponíveis com limites explícitos. Bloquear divergência/corrupção e uso não verificado; sem global install/elevação implícita/SDK installer; plano alterado exige reconfirmação.
- [ ] Exercitar plano pendente/downloading/installing/verifying/ready/cancelled/failed, logs duráveis, cancelamento, retry e parcial não pronta, preservando ferramenta válida. Progresso só medido; sucesso verifica executável/versão, não declara DAP funcional.
- [ ] Mac ARM64 inicia `Apple Silicon debugger: experimental · disabled`, flag OFF, Debug bloqueado e Run independente. Opt-in separado de trust e consentimento não instala nem comprova compatibilidade; só 14/18 podem reunir prova para futura retirada da flag.
- [ ] Mac Intel sem asset 3.2 compatível verificado mostra indisponível e bloqueia instalação/launch, sem baixar outra versão. Windows x64 requer asset adequado e posteriormente smoke DAP próprio. Limites não desabilitam análise/Run/testes disponíveis.
- [ ] Host aplica consentimento/plataforma/flag/trust quando pertinentes, inclusive chamadas diretas. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), incluindo auditoria MIT/avisos e UI acessível. Este ticket independe do instalador csharp-ls.

## Mapeamento à SPEC

AC-04/05/06 (parcial: instalador debugger completo; csharp-ls em 05), AC-22 (parcial: adaptador ausente bloqueia), AC-23 (flag/opt-in), AC-24 (assets/limites; smoke em 14), AC-01/07 e AC-32/33/34 na fatia. SPEC P0.1/P0.5, Avaliação e ferramentas e Licenças.

## Validação

UI real Mac para plano/recusa/flag/estados e instalação oficial somente onde asset compatível existe; ausência legítima exige prova de bloqueio, não download alternativo. Boundary determinística: redirect/origem inválidos, arquitetura errada, path traversal/extração, corrupção/verificação, plano trocado, cancelamento/falha/retry e permissões. Smoke de binário real não substitui DAP; registrar versões/assets/logs e pendências Windows/Intel explicitamente.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
