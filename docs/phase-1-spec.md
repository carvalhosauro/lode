# Phase 1 — Mining Engine (spec técnica)

**Status:** Accepted · alinhado a RFC-0003 §6–§12  
**Tasks:** T1.1–T1.4 em [`TASKS.md`](../TASKS.md)  
**Plano:** [`docs/plans/phase-1-mining-implementation.md`](plans/phase-1-mining-implementation.md)  
**Evidência:** [`docs/experimental/phase1-algorithm-tradeoffs.md`](experimental/phase1-algorithm-tradeoffs.md)

Esta spec traduz a RFC-0003 (o *quê*) em decisões implementáveis (o *como*) para a Phase 1.
Cada decisão está marcada como **Segue RFC**, **Interpretação** (RFC não fixa detalhe, invariante preservado)
ou **Divergência justificada** (default ou escopo alterado com evidência — requer atualização da RFC ou
config explícita documentada).

---

## Como ler RFC vs spec

| Camada | O que define | Exemplo |
|--------|--------------|---------|
| **RFC-0003** | Algoritmo conceitual, invariantes, gate de qualidade | “Mask antes de cluster”; PA ≥ 0.90 |
| **Esta spec** | Estruturas de dados, funções, defaults operacionais | Scanner de tokens; `d=5` para nginx |
| **Código** | Implementação que passa no gate T1.4 | `lode-parse::tokenize`, `lode-core::DrainState` |

A RFC §1 diz explicitamente que **não especifica implementação concreta**. Isso significa:

- Seguir a RFC ≠ copiar um pseudo-código literal.
- Divergir da RFC = violar um **DEC**, **invariante** ou **default documentado sem justificativa**.
- Ajustar um **tunable** (§11) com evidência do corpus **é permitido** pela RFC — desde que documentado.

**Regra prática:** invariantes e gate PA mandam; defaults cedem quando o corpus prova que falham.

---

## RFC alignment & deviations

### Hierarquia de precedência

Quando duas fontes “discordam”, vale esta ordem:

1. **DEC-001…010** e invariante de determinismo (RFC-0003 §6.6) — não negociável.
2. **Gate T1.4:** PA ≥ 0.90 por formato + determinismo (RFC-0003 §12).
3. **Contratos conceituais** (§7): tokenize → mask → fingerprint → route → match → widen.
4. **Defaults §11** (`d=4`, `st=0.5`, …) — tunables; podem ser ajustados por stream ou globalmente.
5. **Detalhe de implementação** (árvore literal vs bucket FNV, char-class vs regex) — livre se semântica igual.

---

### Invariantes — seguimos 100% (nunca divergir)

| Invariante | RFC | O que significa na prática |
|------------|-----|----------------------------|
| Templates derivados | DEC-001 | Ninguém injeta template; só o miner cria. |
| Agrupamento estrutural | DEC-002 | Cluster por tokens mascarados, não por string raw. |
| Mask antes de cluster | DEC-003 | Sempre: tokenize → mask → só então Drain. |
| Árvore de profundidade fixa | DEC-004 | Routing por `length` + primeiros tokens; altura limitada por `d`. |
| Todo evento tem fingerprint | DEC-005 | `Fingerprint` sempre calculado sobre tokens mascarados. |
| Online widen-only | DEC-006 | Ingest só alarga `<*>`; split/merge fica para batch (fora T1). |
| Memória limitada por construção | DEC-008 | `d` fixo, leaf limitado, `T_max` (eviction pode ser stub em T1). |
| Determinismo testado | DEC-009 | Mesma entrada + mesmos parâmetros → mesma saída (por stream). |
| PA ≥ 0.90 | DEC-010 | Gate T1.4; bloqueia merge se não passar. |
| `raw` imutável | RFC-0000 DEC-002 | Tokenizer/masker leem `&str`; nunca alteram o evento. |

---

### Decisões — tabela completa

| # | Decisão Phase 1 | RFC | Status | Justificativa | Evidência | Ação na RFC |
|---|-----------------|-----|--------|---------------|-----------|-------------|
| 1 | Pipeline `tokenize → mask → fingerprint → drain` | §6, §7 | **Segue RFC** | Fluxo idêntico ao §6. | — | — |
| 2 | Tokenizer: scanner estrutural (não `split_whitespace`) | §3.2: *“whitespace **and structural delimiters**, preserving delimiters”* | **Segue RFC** | Whitespace puro **contraria** §3.2; não preserva `[]`, `""`, `{}:,` do corpus. | Spike: whitespace PA 16%; estrutural PA 100%. | — |
| 3 | JSON fast-path (`line.starts_with('{')`) | §3.2 (silêncio) | **Interpretação** | RFC não lista formatos; regra evita que ISO-timestamp de syslog quebre JSON. | Spike: unified sem fast-path → json 49%. | — |
| 4 | Máscaras char-class no hot path | §6.1 (silêncio sobre engine) | **Interpretação** | Dicionário rico §6.1; engine char-class é implementação. | `spike/README.md`: mask ~10× mais caro que tree. | — |
| 5 | Ordem RFC: TS → UUID → IP → … → NUM | §6.1, `MaskKind::EVAL_ORDER` | **Segue RFC** | Most-specific-first explícito. | Já em `lode-core`. | — |
| 6 | Subset T1.1: TS, UUID, IP, HEX, PATH, NUM (sem URL/EMAIL/QUOTED) | §6.1 lista completa | **Interpretação (escopo v1)** | Corpus atual não exige URL/EMAIL; TASK T1.1 lista subset. | Fixtures 3 formatos. | Adicionar em §13 ou nota “v1 subset” se quiser formalizar. |
| 7 | Regras compostas: `<pri>ver→<NUM>ver`, `IP:port→<IP>:<NUM>`, aspas internas | §6.1: *“anchored on token boundaries”*, dicionário rico | **Segue RFC** | Cada regra produz **um token** mascarado; conservador. | Ground truth em `fixtures/corpus/*/templates.json`. | — |
| 8 | Routing: bucket flat FNV `(len, prefix)` | §6.2 árvore conceitual | **Interpretação** | §1 exclui implementação; semântica de routing idêntica. | Spike Rust/Swift: mesmo `template_set_hash`. | — |
| 9 | Similaridade `simSeq = matches / len`; widen-only; routing não widen | §6.2, §6.4 | **Segue RFC** | Texto RFC literal. | — | — |
| 10 | `st = 0.5` | §11 default | **Segue RFC** | Default mantido. | Spike: com `d` correto, st=0.5 passa 100%. | — |
| 11 | **`d = 5` para nginx-access** (global permanece `4`) | §11 default `d=4`; §11 *“per stream or globally”* | **Divergência justificada (per-stream)** | Ver § abaixo — único ponto onde default global falha o gate. | Spike: `d=4,st=0.5` → nginx PA 2%; `d=5` → 100%. | Atualizar RFC §11 nota ou §6.2 exemplo nginx; ou RFC-0016 config. |
| 12 | Registry mínimo; eviction stub até corpus maior | §6.6 `T_max` | **Interpretação (escopo T1)** | Corpus tem ~4 templates/formato; eviction não afeta PA. | Template count << `T_max`. | — |
| 13 | Split crates: parse em `lode-parse`, drain em `lode-core` | §1 (silêncio) | **Interpretação** | `lode-core` dependency-free (workspace rule). | `Cargo.toml` workspace. | — |

---

### A única divergência que importa: profundidade `d`

#### O que a RFC diz

- §11: default global **`d = 4`**.
- §11: parâmetros configuráveis **por stream ou globalmente** (RFC-0016).
- §6.2: layers 2..`d` roteam pelos **primeiros tokens** da linha mascarada.
- §12: PA ≥ 0.90 com **default parameters** — mas §11 trata `d` como **tunable**, não invariante.

#### O problema concreto (nginx)

Linha mascarada (tokens numerados):

```text
[0] <IP>  [1] -  [2] -  [3] <TS>  [4] "GET <PATH> HTTP/1.1"  [5] <NUM>  [6] <NUM>
```

Com **`d = 4`**, o routing usa apenas tokens **`[0..4)`** = `<IP> - - <TS>`.

Esses quatro tokens são **iguais** para GET, POST e DELETE. Todas as linhas nginx caem no **mesmo leaf**.

Dentro do leaf, similaridade entre GET e POST:

- 6 posições iguais de 7 → `simSeq ≈ 0.86` ≥ `st = 0.5` → **match**.
- Widen-only transforma `"GET …"` em `"<*>"` → perde-se GET vs POST vs DELETE.
- **PA nginx ≈ 2%** mesmo com mask perfeito.

Com **`d = 5`**, o routing inclui token `[4]` = `"GET <PATH> HTTP/1.1"` vs `"POST …"` vs `"DELETE …"`.

Leaves **separados** → templates distintos → **PA 100%**.

#### Por que o diagrama da RFC não contradiz isso

O diagrama §6.2 mostra `tok[0] = 'GET'` — um log HTTP **genérico** onde o método é cedo.
O combined log nginx (`IP - - [ts] "METHOD …"`) tem prefixo fixo longo; o discriminador está na posição 4.
Não é bug da RFC; é **formato com prefixo mais longo que o default `d`**.

#### Decisão Phase 1 (proposta fechada)

| Parâmetro | Valor | Escopo |
|-----------|-------|--------|
| `d` | **4** | Default global (`MiningParams::default`) — **mantém RFC §11** |
| `d` | **5** | Override per-stream para `source_type = nginx-access` (RFC §11 per-stream) |
| `st` | **0.5** | Global — mantém RFC §11 |

**Alternativa equivalente** (não escolhida agora): `d=4` global + `st≈0.9` — também passa o corpus,
mas muda merge de forma mais ampla (linhas 86% iguais deixam de fundir). Preferimos `d` per-stream
porque o ajuste é localizado ao formato que exige.

#### Basamento

| Fonte | Resultado |
|-------|-----------|
| Spike descartável | [`phase1-algorithm-tradeoffs.md`](experimental/phase1-algorithm-tradeoffs.md) |
| Golden corpus | `fixtures/corpus/nginx-access/` — 4 templates distinguíveis só no token 4 |
| Gate T1.4 | DEC-010 exige PA ≥ 0.90; `d=4` global falha nginx |

#### Ação documental (RFC)

Antes ou junto com T1.4:

1. **RFC-0003 §6.2** — adicionar nota: combined logs (nginx) podem precisar `d` maior que logs
   onde o verbo HTTP é `tok[0]`.
2. **RFC-0016** (quando existir config) — exemplar `mining.depth = 5` para nginx-access.
3. **Não alterar** default global `d=4` até validação em amostra Loghub maior (Phase 6 / corpus expandido).

---

### O que **não** é divergência (confusões comuns)

| Parece divergir | Na verdade |
|-----------------|------------|
| Bucket FNV em vez de árvore literal | Implementação equivalente (RFC §1). Routing key = `(len, tok[0..d))`. |
| Regras `<NUM>1`, `<IP>:<NUM>`, aspas | Extensões do dicionário §6.1 — ainda em token boundaries. |
| Char-class em vez de regex | Escolha de engine; máscaras pluggable §13 / RFC-0010. |
| Defer URL/EMAIL/eviction/lifecycle | Escopo Phase 1; RFC prevê, TASK não exige para gate T1.4. |
| JSON fast-path | Otimização/ correção de tokenizer §3.2; invariantes intactos. |

---

## Decisões fechadas para implementação

Resumo operacional (referência rápida para T1.1–T1.3):

### T1.1 — `lode-parse`

- `tokenize(raw) -> Vec<Token>` — scanner estrutural + JSON fast-path.
- `mask(tokens) -> MaskedTokens` — ordem RFC; char-class; regras compostas § tabela #7.
- `tokenize_and_mask(raw)` — atalho para pipeline e testes.

### T1.2 — `lode-core`

- `DrainState::process(&MaskedTokens) -> ProcessResult` — bucket FNV, `simSeq`, widen-only.
- `MiningParams::default()`: `d=4`, `st=0.5`, `max_templates=10_000`, `stable_threshold=5`.
- Override `d=5` aplicado no `DrainMiner` quando `source_type == nginx-access` (até RFC-0016).

### T1.3 — wire + fingerprint

- `Fingerprint::from_masked_tokens` (já existe).
- `DrainMiner` em `lode-parse` implementa `CorpusMiner`.
- Gate T1.4: remover `#[ignore]` em `corpus_pa_meets_floor`.

---

## Escopo explicitamente fora de T1 (mas previsto na RFC)

| Item RFC | Phase alvo |
|----------|------------|
| URL, EMAIL, QUOTED masks | T6.1 / RFC-0016 |
| Batch re-mine (split/merge) | Pós-segment seal (Phase 2+) |
| Eviction `T_max` completa | T6.1 |
| Lifecycle events (`mining.template.*`) | T3.3 |
| Regex masks custom (plugins) | RFC-0010 |

---

## Referências

- [RFC-0003 — Template Mining System](../RFC/RFC-0003-Template-Mining-System.md)
- [Golden corpus](../fixtures/corpus/)
- [Spike trade-offs (evidência)](experimental/phase1-algorithm-tradeoffs.md)
- [Spike Rust throughput](../spike/README.md)
- Domain types: `crates/lode-core/src/mining/mod.rs`, `MiningParams::default`
