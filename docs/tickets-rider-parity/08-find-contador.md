# 08 — Contador no Find do editor (G07)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Navegação e Find do editor". **Bloqueado
por:** nenhum. **Status:** ready-for-agent.

## Objetivo

O painel de Find do editor mostra "n de m" ocorrências para a consulta atual, atualizando
quando a consulta, o documento ou as opções mudam.

## Pontos de código

- `src/CodeEditor.tsx`: extensão de busca do CodeMirror (`@codemirror/search` já é dependência),
  `basicSetup`, `gotoLine`, tema do editor.
- `src/styles.css`: aparência do contador no painel.
- `package.json`: nenhuma dependência nova.

## Comportamento

- O painel mantém os controles atuais (próximo, anterior, todos, caixa, regex, palavra,
  substituir, substituir tudo) e ganha o contador `n de m` (1-based quando há ocorrência;
  `0 de 0` sem consulta/ocorrência).
- O contador reflete a consulta vigente, o texto do documento e as opções de caixa/regex/palavra.
- O escopo "na seleção" **fica adiado dentro deste ticket**: escopo com replace exige comandos
  próprios e testes de seleção/replace; o motivo e o que falta ficam registrados abaixo. Não
  exibir um checkbox que não funciona.
- Nada de mudar o atalho ⌘F nem o comportamento de abrir/fechar o painel.

## Adiado: "na seleção"

Motivo: limitar next/prev/all/replace a um range de seleção exige comandos próprios sobre
`SearchCursor`, tratamento de seleção que muda e interação com replace; é P2 e o risco não
cabe no mesmo ticket. O que falta: um escopo explícito no estado da busca, comandos de
navegação/replace limitados ao range e testes de seleção. Registrar em `docs/decisions.md`.

Registrado em `docs/decisions.md` (G07). O painel não exibe nenhum checkbox de "na seleção";
os únicos checkboxes continuam sendo caixa, regex e palavra.

## Teste e verificação

- Playwright: abrir o Find, digitar uma consulta que existe em N lugares, afirmar `n de m`;
  mudar para regex/caixa/palavra e conferir a contagem; apagar a consulta e conferir `0 de 0`.
- App real (agent-desktop): abrir `Order.cs`, ⌘F, digitar `order`, conferir o contador e
  navegar com próximo/anterior.

## Aceite

- [x] O painel mostra `n de m` para a consulta atual.
- [x] O contador atualiza com consulta, documento e opções.
- [x] Sem consulta/ocorrência, mostra `0 de 0`.
- [x] Os controles existentes continuam funcionando.
- [x] "na seleção" está registrado como adiado, sem checkbox falso.
- [ ] Evidência no app real.
