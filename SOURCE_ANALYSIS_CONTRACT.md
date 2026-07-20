# Source Analysis Contract

## Status And Scope

This document defines the developer-facing contract for parsing one exact
`.puzzle` or `.puzzle3` source snapshot into editor-facing surface information.
It covers the shared language boundary used by highlighting, outline,
source-target resolution, and completion. It also defines the boundary between
that tolerant surface analysis and strict compilation.

This contract does not choose an editor library, prescribe a wire format, or
make editor recovery authoritative for runtime semantics. CodeMirror or another
editor may consume the products defined here, but the language contract must not
expose editor-library types.

The normative terms **must**, **must not**, **should**, and **may** describe
requirements for implementations of this contract.

## Objective

One source revision must have one canonical surface interpretation. Different
editor features are projections of that interpretation, not independent
opportunities to reinterpret the source.

The contract separates six concerns:

1. capturing the exact source revision;
2. lexing lossless tokens and trivia;
3. parsing those tokens into owner-scoped syntax;
4. resolving declarations and references;
5. lowering valid syntax into runtime models;
6. projecting already-recognized facts for editor consumers.

Consumer policy may differ. Surface interpretation must not.

The governing rule is stronger than sharing a cache: every source spelling that
the compiler accepts must be recognized by exactly one parser-owned operation.
Highlighting, completion, outline, source targeting, and strict compilation may
select or reject parser facts, but none may recognize the spelling again.

## Terms

### Source revision

A source revision is an immutable revision identity plus the exact source text
and source profile (`.puzzle` or `.puzzle3`) that identity denotes. A service may
reuse an existing revision identity only when both the source text is
byte-for-byte identical and the source profile is unchanged. Any textual or
profile change creates a different revision before new analysis products are
exposed.

Revision numbers are coordination metadata, not source semantics. The language
crate may represent a revision as an immutable `SourceAnalysis` value while a
host or adapter assigns the externally visible revision number.

### Current surface document

The current `SurfaceDocument` is a migration-era representation of
editor-facing syntax. Useful fields include:

- physical lines and their source locations;
- tokens and half-open source spans;
- owner-scoped structural blocks, nesting, and parent relationships;
- recognized leaf or owner roles needed by downstream products;
- recoveries or errors that affect the interpretation of those facts.

It is not the target authority because strict parsing and several owner-specific
recognizers still operate beside it. The canonical target is the parse snapshot
defined below.

### Product and projection

A product is typed information stored in or derived from the canonical parse
snapshot, such as semantic tokens or completion symbols. A projection is a
consumer-facing view derived from canonical products, such as an outline tree or
the source target containing a cursor offset.

Producing a projection may build an index or filter existing data. It must not
parse source text again or introduce a second grammar interpretation.

### Analysis profile

An analysis profile is an optimization that may omit products a caller will not
request. A profile is not a grammar mode. It must not change token boundaries,
block ownership, nesting, recovery, or the meaning of any product it does
produce.

## Target Parser Architecture

### Canonical parse snapshot

The target authority is a lossless parser-frontend product named here
`ParseSnapshot`. It represents one exact source revision and is consumed by both
editor services and strict compilation. It is not an editor approximation of a
separate strict parse.

The snapshot contains:

- lossless tokens, trivia, and half-open spans covering every source byte;
- a recovered concrete syntax tree with owner-scoped nodes and parent links;
- one terminal disposition for every non-whitespace token: recognized syntax
  role, owner payload role, trivia, or explicit error;
- typed declarations, references, display facts, and diagnostics emitted by the
  parser operation that recognized them;
- stable node identities or fingerprints for incremental reuse.

`SurfaceDocument` and `SurfaceSourceScan` are migration structures. They do not
satisfy this authority while strict parsing, owner-specific surface recognition,
or highlighting can reinterpret source text outside the canonical frontend.

### Recognition and grammar ownership

The parser frontend has four recognition stages:

1. A lossless lexer emits only context-free classes such as whitespace,
   comment, quoted string, delimiter, and bare atom.
2. The structural parser assigns ownership and builds recovered syntax nodes.
3. The parser for each owner consumes its token slice and emits contextual
   syntax roles, owner payload facts, and diagnostics.
4. Semantic resolution joins declarations and references without recognizing
   their spelling again.

Generic document structure, authoring rows, rules, levels, visuals, scenes, and
other leaf languages may have distinct owners. An owner receives lexer tokens or
an explicitly opaque token slice from its parent. It may inspect those token
payloads but must not scan the whole source independently.

The executable grammar operation that accepts a token must also attach its role.
Declarative grammar tables are allowed only when parsing, completion
expectations, and syntax roles are derived from the same table. Parallel keyword
lists, `record_*_surface` functions, `scan_*_surface_ranges` functions, and
post-parse spelling classification are forbidden even in parser-named modules.

Contextual classes such as selector marks, directions, declaration names, level
cells, visual pixels, and references are parser facts, not lexical guesses.
Highlighting is a total exhaustive mapping from dispositions and resolved facts
to display kinds. A highlight-only "canonical lexer" that strict parsing does
not consume remains a second grammar implementation.

### Tolerant and strict policy

The frontend always returns a recovered snapshot and diagnostics. Editing may
project useful facts from incomplete nodes. Strict compilation rejects blocking
diagnostics and requires complete typed owner products before lowering. These
are policies over the same parse result, not separate grammar modes.

Parser failure is data, not absence. Lazy products return facts plus diagnostics
or a typed unavailable reason. They must not use `.ok()?`, `Option<Catalog>`, an
empty collection, or a default dimension to hide failed recognition.

### Incremental computation

The editor owns one mutable analysis session. Each ordered edit publishes a new
immutable snapshot which may share unchanged token and syntax storage with its
predecessor. A query observes one complete revision or fails as stale.

Recalculation follows grammar dependencies:

1. Relex from the first affected checkpoint until both token sequence and lexer
   state converge with an unchanged suffix.
2. Reparse the smallest enclosing owner whose input token slice changed. If its
   ownership boundary changes, expand to parents until boundaries stabilize.
3. Re-resolve only symbols and owner products whose dependency keys changed.
4. Re-lower only changed runtime models and their dependents.
5. Reuse projection indexes for unchanged stable node identities.

Whole-document work is allowed when an edit changes global ownership or symbol
meaning. It must be visible in counters rather than hidden behind stale reuse.
Comment-only reuse is proven by an unchanged non-trivia token fingerprint, not
by checking whether edited text appears after `//`.

Tokens, diagnostics, and display facts remain source-sorted in compact arenas or
equivalent contiguous storage. Syntax nodes refer to token ranges rather than
cloning normalized line strings. Owner products are keyed by stable node
identity and input fingerprint.

Viewport highlighting binary-searches ordered disposition and display-fact
indexes and maps only intersecting facts. Its expected cost is `O(log n + k)`;
it never constructs a whole-document highlight result first. Outline, fold,
completion-symbol, target, and semantic indexes may be lazy, but each is built
at most once per snapshot and invalidated by explicit node or symbol
dependencies.

## Canonical Parse Requirements

### One interpretation per revision

For one active source revision, `SourceAnalysis` must own or reference one
canonical parse snapshot. Highlighting, outline, target resolution, completion,
strict validation, and lowering query that snapshot or lazily derived products
attached to it. They must not each call a source-to-document entrypoint.

"One parse" means one recognition of authored spelling. It does not mean every
projection is eager or that semantic resolution and runtime lowering run when a
caller requests only lexical display facts. Laziness may postpone products; it
may not create another recognizer.

### Total analysis for editing states

Canonical frontend parsing must return a snapshot for every source string,
including empty source and incomplete text produced between keystrokes. It must
not require the source to compile successfully.

Recovery must obey these invariants:

- Source text must not be silently rewritten or replaced by guessed text.
- Synthetic boundaries introduced for recovery must be distinguishable from
  authored boundaries.
- A local incomplete construct should not hide later well-formed siblings when
  an unambiguous resynchronization boundary exists.
- A recovered construct must not be presented as strict compile success.
- When recovery leaves two plausible owners or targets, projections that require
  one owner or target must report no unambiguous result instead of choosing one.

The implementation may refine its recovery algorithm, but these observable
properties must remain stable. Recovery behavior that affects block ownership or
source spans requires focused fixtures before it changes.

### Ownership and leaf syntax

The canonical surface document must preserve owner-scoped tree structure.
Owner-specific leaf syntax may contribute typed products through the owning
language implementation, but a generic surface walker must not infer a leaf kind
from appearance.

An optimization may leave an owner-specific leaf body opaque until one of its
products is requested. That laziness must not alter the surrounding tree or the
leaf's authored span.

### Determinism and ordering

The same source bytes, source profile, and language version must produce equal
canonical facts and equal projections. Authored order must be preserved for
ordered constructs. Sets or maps used only for lookup must not make serialized
product order nondeterministic.

## Offset Contract

All parser-owned spans must use zero-based, half-open UTF-8 byte offsets into the
exact source revision:

```text
[start, end)
```

Every span boundary must lie on a UTF-8 character boundary and must satisfy
`0 <= start <= end <= source.len()`.

Browser UTF-16 offsets are adapter data. Conversion between UTF-8 byte offsets
and UTF-16 code-unit offsets must happen at the host or editor boundary against
the same exact source revision. Converted offsets from a stale or different
revision must not be applied.

No language product may mix byte offsets and UTF-16 offsets in the same field or
reuse an unlabeled numeric offset across that boundary.

## Required Projections

### Highlighting

Highlighting must derive semantic token kinds and special display ranges from
the canonical surface document. The language boundary exposes typed
`SourceHighlightSpan` values with UTF-8 byte ranges, `SourceHighlightKind`, and
optional display data such as a color or transparency flag. Editor wire version
3 serializes those spans with `offsetEncoding: "utf8"`, the exact source byte
length, and the requested `[start, end)` byte range. A range response contains
only spans intersecting that range; intersecting spans retain their complete
token boundaries. HTML and CSS class names must not cross the language boundary;
an adapter may render local HTML or map kinds to editor decorations after
consuming the typed product.

The editor must request the visible source range plus bounded overscan. It must
map existing decorations through edits and replace only the returned range,
rather than requesting or rebuilding a whole-document presentation after every
keystroke. Viewport selection is presentation state owned by the editor; token
recognition within that range remains language-owned.

Browser source-analysis queries run in one dedicated worker and share its active
source revision. Highlight, outline, completion, and target queries must not run
Rust parsing synchronously on the browser main thread. Worker failure is a
visible analysis failure; the editor must not fall back to a JavaScript grammar
or to main-thread source analysis.

The active analysis is a long-lived document session. CodeMirror initializes it
with one exact source snapshot, then sends ordered UTF-16 edits as
`{ from, to, insert }`; the WASM boundary converts those edits to UTF-8 before
updating the parser-owned source. Ordinary typing must not reactivate analysis
from a newly transferred full source string. Each accepted edit advances the
analysis revision, and queries against an earlier revision fail visibly.

One revision owns one canonical parse snapshot. Highlight, outline, completion,
entries, targets, and strict compilation consume that snapshot rather than
building product-specific documents. Incremental lexer, owner, symbol, and
lowering products may survive only when their recorded input fingerprints and
dependencies are unchanged; otherwise they are invalidated explicitly.

Highlighting may display recovered or incomplete tokens. It must not claim that
their enclosing construct is valid merely because it can assign a color.

Highlight projection completeness is an editor-assistance contract, not a
language-acceptance contract. Missing semantic color or display classification
must be reported by the highlight product and must not reject compilation,
preview, or export when parser diagnostics and lowering otherwise permit those
operations. Strict compilation may reject a missing canonical token disposition
only when that disposition is owned by the parse snapshot itself; it must not
infer the failure from highlight spans, colors, or display-range coverage.

The Rust highlight module is a projection consumer. It must not scan source
characters or own token, comment, quote, brace, selector, or owner-leaf
recognition. The parser frontend stores lossless token dispositions, semantic
facts, and owner-produced display facts on the snapshot. The typed highlight
product may only map those facts. In Rust, parser recognition owns
`ParserTokenDisposition`; `SurfaceSemanticToken` is a separate editor-facing
projection and must never be merged back into parser recognition. The
highlight API must not accept source text.

Range highlighting must locate intersecting lines, facts, semantic tokens, and
owner ranges through source-sorted indexes and map only that window. It must not
build, scan, or filter a whole-document highlight span product for a viewport
query. A missing display fact is therefore a missing canonical surface contract,
not permission to add a recognizer to highlighting.

### Outline

Outline items must derive labels, kinds, hierarchy, and source spans from
canonical structural blocks. Outline generation must not rescan brace lines or
reconstruct ownership from highlighted HTML.

The Rust outline module is a projection consumer and must not inspect source
text, block headers, physical line content, grammar tables, scopes, or syntax
word lists. The parser's existing structural pass must attach typed outline kind,
label, and child policy to canonical blocks without adding another full-source
pass. A parser-owned outline product may then assemble those facts lazily and be
cached on the active `SourceAnalysis` revision. That lazy product must be built
at most once per revision, only when outline is requested; highlight-only and
other non-outline queries must not construct it. An editor outline request must
query the already-active profile-aware analysis revision rather than submitting
the source to a separate outline parser.

An outline item may be omitted when its owner or label is ambiguous. Items after
a recoverable local error should remain available when their canonical blocks
remain available.

### Source targets

Source entries and cursor targets must derive from canonical blocks, lines,
owner roles, and owner-produced references. Resolving a target may build a
revision-local interval index. It must not run a separate source parser.

Target resolution must return no target when the cursor belongs to no supported
entry or when recovery leaves the target ambiguous. It must not fall back to a
nearby entry, a previous revision, or a name-based guess.

### Completion

Completion context must derive from canonical lines, tokens, scopes, blocks, and
the cursor offset. Completion symbols must be a canonical product of the same
revision, even when calculated lazily.

Completion may interpret an incomplete node as incomplete. It must not mutate
the canonical document by pretending candidate text was authored. Speculative
parsing of a candidate insertion, when needed to rank or validate that candidate,
is a completion operation on hypothetical text and is not the canonical parse of
the active source revision.

## Profile Equivalence

`FULL`, `STRUCTURE_ONLY`, `SOURCE_TARGET`, `COMPLETION_SYMBOLS`, or future
profiles may exist only as product-selection optimizations.

For every source and every product selected by a profile, that product must equal
the corresponding product of the full canonical analysis. In particular:

- lines and structural blocks produced by any profile must equal the full
  document's lines and structural blocks;
- source-target references produced by a target profile must equal the full
  document's references;
- completion symbols produced by a completion profile must equal the full
  document's completion symbols;
- semantic tokens and highlight ranges produced by a highlighting profile must
  equal the full document's products.

If a reduced profile cannot satisfy this equivalence without using a different
recovery or interpretation, the profile must be removed or the differing result
must be named as a different language operation rather than a profile.

Within one `SourceAnalysis`, reduced profiles must not cause repeated canonical
surface construction. Laziness belongs inside the canonical document or its
revision-local product cache.

## Revision And Concurrency Contract

Every externally returned analysis result must identify the source revision it
describes. A host or editor must apply a result only when both the document
identity and revision still match the active source.

Analysis cancellation is an execution concern. Cancellation may prevent an
unneeded projection from completing, but must not expose a partially mutated
canonical document. An activated `SourceAnalysis` is immutable after publication;
lazy caches may initialize internally only when their observable result is
deterministic.

Hosts may coalesce work so only the latest not-yet-started revision is analyzed.
They must not answer a newer revision with products cached from an older source.
Exact source equality may reuse the same immutable analysis.

## Strict Compile Boundary

The canonical parse snapshot is authoritative for syntax recognition, but a
recovered snapshot is not proof that the game compiles or has runtime meaning.
Strict compilation is a policy and lowering pipeline over that snapshot:

1. reject every blocking lexical, syntax, ambiguity, and resolution diagnostic;
2. require the complete typed owner products needed by the runtime target;
3. lower those products into runtime models.

Strict compilation must not accept source text after snapshot construction and
must not invoke a second lexer, structural normalizer, owner parser, or catalog
parser. Tolerant editing and strict compilation differ only in what diagnostics
and incomplete products they permit downstream; they do not differ in grammar
recognition.

The editor must not treat surface recovery as a compatibility path around strict
compile failure. Preview and export must continue to fail visibly with the strict
diagnostics.

## Host And Editor Boundary

Language APIs must return typed source facts: spans, kinds, hierarchy, symbols,
targets, diagnostics, and revision identity. They must not return CodeMirror
decorations, DOM nodes, editor selections, or other adapter-owned values.

The editor adapter owns:

- conversion to editor-library ranges and decorations;
- viewport rendering;
- cursor and selection state;
- stale-result rejection using document identity and revision;
- UTF-8/UTF-16 conversion at the boundary;
- presentation of unavailable or ambiguous products.

The editor must not tokenize source, recognize owner blocks, resolve names, or
repair missing language products in JavaScript.

## Verification Requirements

Implementation of this contract is incomplete without focused tests for the
following properties.

### Canonical and profile equivalence

- Each reduced profile matches the full document for every product it selects.
- Highlight, outline, target, and completion entrypoints consume one activated
  analysis rather than constructing independent surface documents.
- Exact source reuse returns the same revision-local analysis products.

### Incomplete-source recovery

Fixtures must cover at least:

- unmatched opening and closing braces;
- an unterminated quoted string;
- an incomplete assignment;
- a half-authored block header;
- an incomplete owner-specific leaf body;
- a valid sibling following each local error.

Tests must assert spans, ownership, later-sibling visibility, ambiguity behavior,
and strict compile failure where applicable. Snapshot-only tests are insufficient
when they do not assert those contracts explicitly.

### Offset and revision safety

- Multibyte BMP characters and supplementary characters round-trip between
  parser byte offsets and browser UTF-16 offsets.
- Applying a product to a different source revision is rejected.
- No span splits a UTF-8 character.

### Boundary enforcement

- Editor JavaScript does not implement a second source grammar.
- Presentation helpers do not become canonical language products.
- Tolerant analysis does not let preview or export bypass strict diagnostics.
- Every accepted non-whitespace token has exactly one parser-owned disposition.
- Strict compiler entrypoints consume a parse snapshot and cannot access source
  text.
- Highlight, outline, and fold modules cannot access source text or raw token
  payloads through their APIs. Completion and target projections may read
  authored text only through typed tokens; they cannot split or classify it.
- Adding a grammar rule without a syntax disposition fails a grammar coverage
  test or an exhaustive type check.
- Editing one owner body reparses that owner and its proven dependents, not
  unrelated sibling owners.
- Performance benchmarks record relexed tokens, reparsed owner nodes,
  re-resolved symbols, lowered models, and returned projection facts rather than
  only wall-clock time.

## Migration Gate For The Source Editor

This specification is the contract gate before language-aware CodeMirror
integration. A plain-text CodeMirror shell may be developed against document,
selection, change, undo, and viewport APIs while this contract is implemented.

Semantic highlighting, outline, target navigation, and completion must not be
migrated by adapting the current full-document HTML output into CodeMirror. They
must consume typed projections satisfying this contract. The old textarea
overlay and the new editor must not remain as long-lived runtime alternatives;
the final cutover must delete the old source-rendering, undo, folding, and caret
geometry paths once feature and performance gates pass.

Parser migration is performed as atomic owner slices, not by adding a new global
surface layer beside the old parser. For one owner at a time, the same change
must:

1. make the canonical frontend produce its typed nodes, dispositions, display
   facts, and diagnostics;
2. make strict validation/lowering and editor projections consume those facts;
3. delete the corresponding raw-source parser, `record_*_surface`,
   `scan_*_surface_ranges`, keyword list, and compatibility path;
4. add specification fixtures and incremental invalidation tests;
5. show that production recognition code did not preserve the replaced
   recognizer under a new name.

A migration slice is incomplete when production recognition code is net-added
without deleting the prior owner implementation. Net growth is allowed only for
new information that had no previous owner, such as stable node identity or a
dependency index, and review must identify that information explicitly.

Recommended execution order:

1. Replace structural normalization and the editor line scanner with the
   lossless lexer plus recovered document tree. Keep full owner reparsing at
   first, but retain lexical checkpoint convergence. Delete both old structural
   implementations in this slice.
2. Move declarative authoring blocks to executable grammar descriptors that emit
   nodes, dispositions, completion expectations, and diagnostics. Delete their
   surface projection recognizers.
3. Move rule syntax, including patterns and control flow, and make 2D/3D lowering
   consume the shared typed rule product.
4. Move levels and legends, then visuals and visuals. These owners justify opaque
   token slices and owner-local incremental parsers because their ASCII bodies
   have different lexical meaning.
5. Move scenes, metadata, imports, and remaining leaf owners; then delete the
   generic surface-recognition layer.
6. Add owner-subtree and symbol-dependency reuse only after each owner has one
   authoritative parse. Optimization must not cache a duplicate recognizer.

Each step first proves a full-parse result from the new authority, then enables
incremental reuse for that same result. This separates semantic equivalence from
cache correctness and prevents a stale cache from making two parsers appear
equivalent.

The migration finishes only when:

- `source_canonical_lexical.rs` or any equivalent highlight-only lexer is gone;
- `lib_surface_doc.rs` contains projections/builders but no source recognizers;
- strict compile accepts a parse snapshot rather than source text;
- optional parser catalogs and raw-source rescans are gone;
- a repository search finds one executable grammar owner for every syntax
  family and zero grammar recognizers in projections or adapters.
