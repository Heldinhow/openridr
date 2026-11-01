# 03: Trust prompt modal na primeira abertura (UI)

**What to build:** Ao abrir um projeto sem decisão (seletor, recente ou formulário de caminho) aparece um modal único com Trust project, Open in Restricted mode e Cancel; projeto com decisão abre direto. O card Review workspace trust some.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] Modal acessível: foco inicial em Trust project, Esc=Cancel, Enter no botão focado, Tab circula, foco preso e devolvido
- [ ] Cancel volta à welcome sem abrir; as outras duas gravam a decisão e entram
- [ ] Reabrir o mesmo projeto não mostra o prompt (E2E com OPENRIDR_HOME isolado)
- [ ] Specs que passavam pelo card/etapa de trust migrados; testids de TrustControls preservados
- [ ] `bun run check` e specs afetados verdes; commit `feat(trust): ...`

## Comments
