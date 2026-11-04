---
name: test-app-agent-desktop
description: Receitas testadas para dirigir o app OpenRidr real no macOS com agent-desktop (achar elementos por ref, digitar no editor, atalhos com foco, diálogos de saída, duas instâncias, capturas, limpeza). Use junto com verify-openridr quando for preciso clicar, digitar ou apertar atalho no app desktop, ou quando uma receita de agent-desktop falhar.
---

# test-app-agent-desktop

Complementa `verify-openridr` (superfície C). Tudo aqui foi executado contra OpenRidr 0.3.0-alpha.5 em macOS 27.2 arm64, com agent-desktop 2.5.

Antes de qualquer coisa: a tela precisa estar desbloqueada, e o app que roda o agent-desktop precisa de Acessibilidade. `agent-desktop status` deve mostrar `accessibility: granted` e `screen_recording: granted`.

## Regras de segurança

- Nunca `pkill`/`killall` por nome. Use o PID que você subiu, ou `osascript -e 'tell application "<caminho do .app>" to quit'`.
- `OPENRIDR_HOME` sempre apontando para `/tmp/...`. Nunca para `~/Library/Application Support/OpenRidr`.
- Antes de qualquer `--headed` (teclado, digitação): `agent-desktop focus-window --app OpenRidr`. O `--headed` move o foco e o cursor do usuário.
- `--force` (ex.: Cmd+Q) só em teste isolado e com dado descartável.
- Não clique em rótulo curto ("Close", "Save"): a busca por substring pega o primeiro que bater. Use correspondência exata (ver o helper).

## Ciclo básico

1. Snapshot pequeno para achar o que interessa: `agent-desktop snapshot --app OpenRidr -i --compact` (só interativos).
2. Snapshot completo quando precisa do **valor** de um campo ou de `statictext` (ex.: texto do editor, "Ln 3, Col 1", "modified"): `agent-desktop snapshot --app OpenRidr` (sem `-i`).
3. Refs são `@<snapshot_id>:eN` e valem só para o snapshot que os gerou. Pegue o ref de um snapshot novo a cada ação.
4. Ações: `click @ref`, `focus @ref`, `right-click @ref`, `type @ref "texto"`, `set-value @ref "v"`.

Helper mínimo (resolve o ref por role+nome num snapshot novo e age; `HEADED=1` adiciona `--headed`):

```python
#!/usr/bin/env python3
# usage: [HEADED=1] act.py ROLE NAME VERB [ARG]   NAME "=x" = exato; sem "=" é substring
# usage: [HEADED=1] act.py - - press COMBO
import json, os, subprocess, sys
role, name, verb = sys.argv[1], sys.argv[2], sys.argv[3]
arg = sys.argv[4] if len(sys.argv) > 4 else None
extra = ["--headed"] if os.environ.get("HEADED") else []
run = lambda c: (lambda p: (p.stdout + p.stderr).strip())(subprocess.run(c, capture_output=True, text=True))
if role == "-" and verb == "press":
    print(run(["agent-desktop", "press", arg, "--app", "OpenRidr"] + extra)); sys.exit(0)
tree = json.loads(run(["agent-desktop", "snapshot", "--app", "OpenRidr"]))["data"]["tree"]
found = []
def walk(n):
    nm = n.get("name") or ""
    ok = nm == name[1:] if name.startswith("=") else name.lower() in nm.lower()
    if n.get("role") == role and ok and n.get("ref_id"): found.append(n["ref_id"])
    for c in n.get("children", []): walk(c)
walk(tree)
if not found: print("NOTFOUND", role, name); sys.exit(3)
print(run(["agent-desktop", verb, found[0]] + ([arg] if arg is not None else []) + extra)[:500])
```

Valores de campo: `grep` no snapshot completo, por exemplo `textfield: Editor = '…'`. Confirme sempre no disco também.

## Receitas que funcionam

- **Abrir solução**: `OPENRIDR_HOME=/tmp/x nohup <bin> /caminho/OrderDesk.sln &`. Trust aparece como `dialog: Trust “…”?` com `button: Trust project` / `Open in Restricted mode` / `Cancel`.
- **Editar e salvar**: `focus-window`, depois `act.py textfield Editor focus`, depois `HEADED=1 act.py textfield Editor type "x"` (ver falhas). `act.py button "=Save" click` grava. Confirme com `head`/`grep` no arquivo.
- **Atalhos**: `HEADED=1 act.py - - press cmd+s`, `cmd+z`, `cmd+b` (go to definition) funcionaram com o editor focado.
- **Fechar aba com alteração**: `act.py button "=Close Nome.cs" click` → `dialog: Close modified file?` com `Save` e `Discard and close`. Escape mantém a aba.
- **Fechar a janela com alteração**: botão Close da janela (`act.py button "=Close" click`, **só** com o Settings fechado, senão fecha o app) → `Close OpenRidr with unsaved changes?` com `Save all`, `Keep editing` e `Discard and close app`.
- **Menus de contexto**: `act.py button "Nome ·" right-click` abre `menu:`. Feche com `press escape` duas vezes antes de outra ação; um menu aberto faz as próximas ações falharem com `ACTION_FAILED`.
- **Build/Run/Test**: clique na toolbar (`=Build`, `=Run`) ou nas abas (`radiobutton`). Espere pelo estado com `sleep`: build ~45–60 s (com restore), run ~30 s, discover ~60 s, run all ~100 s, no fixture OrderDesk.
- **Capturas**: `agent-desktop screenshot --app OpenRidr docs/evidence/<tema>/x.png`. Só com uma instância viva (ver falhas).
- **Abrir pelo CLI do projeto**: `OPENRIDR_APP=<bundle> sh scripts/openridr <caminho>` usa `open -a`, que reutiliza a instância única. Como LaunchServices não repassa `OPENRIDR_HOME`, use `launchctl setenv OPENRIDR_HOME /tmp/x` antes e `launchctl unsetenv OPENRIDR_HOME` depois.

## Falhas conhecidas (e o que fazer)

| Sintoma | Causa | Contorno |
|---|---|---|
| `type` no Editor sem `--headed`: `ACTION_FAILED … post_action_verification`, `delivered_unverified`, texto não entra | Entrega semântica não chega ao CodeMirror | `HEADED=1` (com `focus-window` antes) |
| `press cmd+s` sem `--headed`: "Application has no verified focused element for keyboard delivery" | Sem foco verificado | `--headed` depois de `focus-window --app OpenRidr` |
| `press cmd+q`: `POLICY_DENIED` ("dangerous shortcut") | Bloqueio de atalho perigoso | `--force`, só em teste isolado |
| `close-app` / `screenshot` com duas instâncias: `AMBIGUOUS_TARGET` | Mais de uma `openridr` | Não rode duas. Feche com osascript pelo caminho do bundle. `snapshot --window-id` funciona; `screenshot` com `--window-id` não |
| Duas instâncias ao executar o binário direto | Binário direto não passa por LaunchServices | Feche antes; use `open -a` para reutilizar |
| `press cmd+f`: `APP_UNRESPONSIVE menu_modifier_value_invalid` | Erro da tool ao mandar Cmd+F | Não resolvido. Não testado: buscar o item no menubar (`snapshot --surface menubar`) |
| Foco de teclado (Tab) não aparece | Conteúdo web: a árvore vê só `webarea` | Não verificável por esta superfície; registre "não verificado" |
| Disclosure ("Project: OrderDesk.sln") sem `ref_id` | Não exposto como ação | Use outro caminho (ex.: Search Everywhere) ou registre como não alcançado |
| `resize-window`: `TIMEOUT … resolution_deadline_exhausted` (passo `window_resize` diz succeeded) | Tool | Confira o tamanho por captura; não confie só no código de saída |
| `right-click` com menu já aberto: `ACTION_FAILED` | Menu anterior ainda ativo | `press escape` antes |
| `click` retorna `delivered_unverified` | Normal para `AXPress` | Verifique pelo efeito (snapshot, disco, `trusted.json`) |

## Limpeza

- Ao fim: `osascript … quit`, depois `pgrep -lx openridr` vazio.
- `launchctl getenv OPENRIDR_HOME` vazio.
- Estado do teste em `/tmp/…`; a cópia do app em `~/Applications-test/…` pode ficar.
- Nunca deixe instância viva com buffer sujo sem saber: o quit descarta (ver BUG-001 do e2e alpha.5).
