# 03 — b14: recarregar buffer limpo quando o disco muda

Status: ready-for-agent
Achado: QA-BUGHUNT-2026-11-02 §14.

## Aceite

- `checkDisk` com buffer limpo e hash diferente troca `content`/`savedContent`/`hash` pelo texto do disco, sem pedir decisão.
- `checkDisk` com buffer sujo mantém o texto e cria `conflict` (Reload/Overwrite), nunca sobrescreve.
- O reload não conta como edição local nem liga a aba (`*`).

## Teste

E2E: abrir dois arquivos, alterar `Program.cs` no disco, voltar à aba; conteúdo novo aparece. Editar sem salvar, alterar de novo, voltar; conteúdo local permanece e o banner de conflito aparece.
