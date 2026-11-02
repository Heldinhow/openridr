# 06 — b11: NuGet mostra a versão atual real

Status: ready-for-agent
Achado: QA-BUGHUNT-2026-11-02 §11.

## Aceite

- `parse_listing` lê `resolvedVersion`/`requestedVersion` e mantém compatibilidade com `resolved`/`requested`.
- A coluna Current mostra a versão resolvida; o plano de update usa a versão atual correta.
- Nada muda quando o JSON só traz os campos antigos.

## Teste

Rust: JSON real do SDK com `requestedVersion`/`resolvedVersion` para 3 pacotes; os três resolvem. JSON antigo continua passando.
