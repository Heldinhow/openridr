# 11: Compilar contexto e navegar erros no output

**What to build:** Selecionar projeto executável de startup, configuração e TFM e compilar contexto explícito com dotnet real, mostrando output bruto e erros navegáveis sem confundir diagnósticos Build com LSP.

**Blocked by:** 04 — Restaurar explicitamente e recarregar contexto; 06 — Navegar diagnósticos por documento e revisão.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Mostrar startup/configuração/TFM e exigir seleção em ambiguidade; biblioteca não é aceita como startup executável. Build identifica alvo/contexto/comando, SDK do projeto e avaliação válida; não reusar TargetPath/contexto invalidado.
- [x] Build exige trust no host e não executa restore implícito; dependências pendentes apontam à ação explícita de 04. Não salvar/sobrescrever buffers silenciosamente: identificar conteúdo salvo usado e decidir explicitamente qualquer salvamento necessário.
- [x] Transmitir e preservar stdout/stderr, estados, cancelamento e exit conhecido/unknown. Build falho permanece identificado e impede consumidores Run/Debug; corrigir/salvar/recompilar gera nova execução verificável. (Consumo efetivo por Run/Debug fica nos tickets 12/14; o contrato exposto é `state`+`stale`.)
- [x] Interpretar conservadoramente posição em output textual, mantendo linhas não reconhecidas no log bruto, sem assumir JSON universal. Erro com posição abre fonte; sem posição não inventa linha.
- [x] Problems preserva `Build · dotnet`, severidade/código/mensagem/categoria Unknown quando não conhecida, execução/escopo/revisão salvos. Não fundir com `LSP · csharp-ls` apagando origem; editar buffer/contexto marca snapshot Build antigo como obsoleto, não falha corrigida.
- [x] Cancelar/revogar solicita término real da árvore e informa resultado observado/remanescentes; pedido não é prova de saída e exit ausente não vira zero. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com logs e controles acessíveis desde aqui.

## Mapeamento à SPEC

AC-03 (sem restore implícito), AC-10 (contexto), AC-14/15 (parcial: origem/revisão Build), AC-17 (parcial: seleção/Build), AC-18 (parcial: falha/erro/log/rebuild e condição para launch), AC-19 (parcial: snapshot salvo), AC-20 (cancelamento Build), AC-01/07, AC-32/33. Run/Debug efetivos são 12/14.

## Validação

UI Mac com dotnet real: fixture com erro → log/Problems → abrir fonte → corrigir/salvar/rebuild. Testar configuração Debug e mudança de TFM/contexto, buffer divergente e restore pendente sem tráfego/restore escondido. Boundary: linhas não parseáveis, paths com espaços/CRLF, cancelamento com filhos, resposta tardia, revogação direta e exit desconhecido. Registrar comandos/SDK/revisão/logs e evidência ou limitação por ambiente.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e verificado em macOS arm64 com dotnet 8.0.408 real; evidência em [11-build.md](../evidence/11-build.md). Build é de um projeto console por vez; Run/Debug efetivos ficam nos tickets 12/14.
