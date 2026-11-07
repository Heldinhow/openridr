# Retro — open-source readiness (#224)

## O que aprendemos

- A lista de releases é o link correto para alphas: `releases/latest` não seleciona pré-releases.
- Documentar um canal privado não o habilita. Até a configuração pelo mantenedor não há canal privado funcional; a evidência e o PR registram essa janela.
- Nem toda issue com label de ajuda é uma boa primeira contribuição. O conjunto aberto não ofereceu 3–5 candidatas adequadas sem alterar o critério de risco; a meta ficou pendente.
- O reviewer identificou alegações incorretas sobre debug, licenças e canal de perguntas, corrigidas antes da entrega. Evidência e limites estão em [evidence/open-source-ready.md](evidence/open-source-ready.md).

## Melhorias candidatas, por severidade

1. **Acesso à informação:** tornar o estado de Discussions e do reporte privado visível no checklist de abertura pública. Evita confundir um link documentado com um canal ativo; requer decisão do mantenedor, não mudança automática de settings.
2. **Diagnóstico de ferramentas:** devolver causa, horário e identificador de tentativa quando `delegate_task` falhar. Três falhas genéricas em #227 impediram distinguir indisponibilidade de provedor de erro da tarefa; não há dados para afirmar a causa.
3. **Checagens:** considerar validar templates de issue e links locais nos testes já executados pelo CI. `bun run check` cobre tipos e Rust, e o job TypeScript já roda `bun test tests/unit`; hoje a checagem YAML desta frente foi manual, não um gate contínuo.

Nenhuma mudança de harness, CI ou AGENTS.md foi feita nesta frente. App e E2E não foram executados: não se aplicam ao escopo documental.
