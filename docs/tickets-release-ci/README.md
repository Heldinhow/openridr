# Tickets — Release assinado e notarizado pelo GitHub Actions

Fonte: `docs/SPEC-release-ci.md` (issue #44). Tickets são locais (não são issues do GitHub).
Cada fatia exige prova executada; nenhum ticket fecha por inspeção estática.

| Ticket | Entrega | Bloqueado por | Status |
|---|---|---|---|
| 01 | Preflight do release (`release-macos.sh --check`) com self-check | — | ready-for-agent |
| 02 | Corpo das notas com bloco de verificação gerado | — | ready-for-agent |
| 03 | Build + DMG + `--dry-run` com `GITHUB_OUTPUT` | 01 | ready-for-agent |
| 04 | Caminho real: Developer ID, auditoria de Mach-O, notarização, grampo, verificação | 03 | ready-for-agent |
| 05 | Workflow `.github/workflows/release.yml` + actionlint | 01, 02, 04 | ready-for-agent |
| 06 | Docs: `release-signing.md` (CI, cadastro dos secrets, primeiro run), `RELEASE-ALPHA.md`, decisões, índice | 05 | ready-for-agent |

Decisões: `docs/decisions.md` (2026-11-06, “Release CI”). Evidência: `docs/evidence/release-ci.md`.
