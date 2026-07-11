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

The contract separates four concerns:

1. capturing the exact source revision;
2. recognizing its lossless surface structure;
3. deriving typed products for individual consumers;
4. applying consumer-specific policy to incomplete, ambiguous, or invalid
   structure.

Consumer policy may differ. Surface interpretation must not.

## Terms

### Source revision

A source revision is an immutable pair of a revision identity and the exact
source text that identity denotes. A service may reuse an existing revision
identity only when the source text is byte-for-byte identical. Any textual
change creates a different revision before new analysis products are exposed.

Revision numbers are coordination metadata, not source semantics. The language
crate may represent a revision as an immutable `SourceAnalysis` value while a
host or adapter assigns the externally visible revision number.

### Canonical surface document

The canonical surface document is the one parser-owned representation of the
source revision's editor-facing syntax. It preserves enough source facts to
derive all products in this contract without rescanning or reparsing the source.
At minimum those facts include:

- physical lines and their source locations;
- tokens and half-open source spans;
- owner-scoped structural blocks, nesting, and parent relationships;
- recognized leaf or owner roles needed by downstream products;
- recoveries or errors that affect the interpretation of those facts.

The current `SurfaceDocument` is the implementation candidate for this role.
This document specifies the role, not that the current fields are already
complete.

### Product and projection

A product is typed information stored in or derived from the canonical surface
document, such as semantic tokens or completion symbols. A projection is a
consumer-facing view derived from canonical products, such as an outline tree or
the source target containing a cursor offset.

Producing a projection may build an index or filter existing data. It must not
parse source text again or introduce a second grammar interpretation.

### Analysis profile

An analysis profile is an optimization that may omit products a caller will not
request. A profile is not a grammar mode. It must not change token boundaries,
block ownership, nesting, recovery, or the meaning of any product it does
produce.

## Canonical Parse Requirements

### One interpretation per revision

For one active source revision, `SourceAnalysis` must own or reference one
canonical surface document. Highlighting, outline, target resolution, and
completion must query that document or lazily derived products attached to it.
They must not each call a source-to-`SurfaceDocument` entrypoint.

"One parse" here means one construction of the canonical editor-facing surface
document. It does not mean that every projection must be eagerly calculated, or
that semantic validation and runtime lowering are part of the same operation.

### Total analysis for editing states

Canonical surface construction must return a document for every source string,
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

The same source bytes and the same language version must produce equal canonical
facts and equal projections. Authored order must be preserved for ordered
constructs. Sets or maps used only for lookup must not make serialized product
order nondeterministic.

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

One revision owns one full `SurfaceDocument`. Highlight, outline, completion,
entries, and target projections must consume that document rather than building
product-specific surface documents. The line scanner preserves the prefix before
the first changed line and rescans the structurally dependent suffix. Parser
catalog data may survive only when the edit is proven to affect comment text
alone; otherwise it is invalidated explicitly.

Highlighting may display recovered or incomplete tokens. It must not claim that
their enclosing construct is valid merely because it can assign a color.

### Outline

Outline items must derive labels, kinds, hierarchy, and source spans from
canonical structural blocks. Outline generation must not rescan brace lines or
reconstruct ownership from highlighted HTML.

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

The canonical surface document is authoritative for editor-facing surface
facts. It is not by itself proof that the game compiles or that a recovered node
has runtime meaning.

Strict validation, owner-specific parsing, semantic resolution, lowering, and
runtime model construction remain owned by their existing language and compiler
boundaries. They may reject a source for which tolerant surface analysis returned
useful editor products.

Where strict compilation and surface analysis recognize the same syntax fact,
they should converge on the canonical product rather than maintain divergent
grammars. This contract does not require immediate replacement of every existing
strict parser with `SurfaceDocument`; such a migration requires a separate
owner-by-owner specification and equivalence proof.

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
