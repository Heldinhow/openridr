# 05: Embutir Inter e JetBrains Mono

**What to build:** `@fontsource-variable/inter` e `@fontsource-variable/jetbrains-mono` (latin; roman e itálico da mono) importados em `src/main.tsx`, servidos como assets do app (offline). `--sans` → Inter Variable; `--mono` → JetBrains Mono Variable; editor default `JetBrains Mono`; terminal usa o mesmo `--mono`. `font-variant-ligatures: none` no editor e terminal. Itálico real nos comentários. Usuários com `editor.fontFamily` salvo não são migrados (D10).

**Blocked by:** aprovação das dependências (D7).

**Status:** implementado (commit local; ver RETRO-UI-POLISH).

- [ ] `document.fonts` mostra as duas famílias `loaded` com a rede desligada (no app Tauri real).
- [ ] DMG/app antes e depois medidos (esperado ~130 KB de woff2).
- [ ] pt-BR (`ã é ç`) renderiza sem fallback; decidir se `latin-ext` é necessário.
- [ ] Opção "JetBrains Mono" das Settings passa a funcionar em máquina sem a fonte instalada.
- [ ] `=>`, `!=`, `==` sem ligadura; comentários em itálico real.
- [ ] Sem texto invisível no carregamento (`font-display`).
