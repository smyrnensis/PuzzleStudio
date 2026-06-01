use crate::ast::EffectAst;
use crate::loaded::SceneEffect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceSemanticKind {
    Keyword,
    Literal,
    Binding,
    Effect,
    Emission,
    Input,
    State,
    Condition,
    Scene,
    Asset,
    Number,
    String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceSemanticToken {
    pub(crate) span: SourceSpan,
    pub(crate) kind: SurfaceSemanticKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceNodeKind {
    SceneEffect,
    RewriteEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceNode {
    pub(crate) kind: SurfaceNodeKind,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceDocument {
    pub(crate) nodes: Vec<SurfaceNode>,
    pub(crate) semantic_tokens: Vec<SurfaceSemanticToken>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SurfaceSink {
    document: SurfaceDocument,
}

impl SurfaceSink {
    pub(crate) fn node(&mut self, kind: SurfaceNodeKind, span: SourceSpan) {
        self.document.nodes.push(SurfaceNode { kind, span });
    }

    pub(crate) fn mark(&mut self, span: SourceSpan, kind: SurfaceSemanticKind) {
        if span.start < span.end {
            self.document
                .semantic_tokens
                .push(SurfaceSemanticToken { span, kind });
        }
    }

    pub(crate) fn into_document(self) -> SurfaceDocument {
        self.document
    }

    pub(crate) fn has_semantic_tokens(&self) -> bool {
        !self.document.semantic_tokens.is_empty()
    }

    pub(crate) fn extend(&mut self, document: SurfaceDocument) {
        self.document.nodes.extend(document.nodes);
        self.document
            .semantic_tokens
            .extend(document.semantic_tokens);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceSceneEffect {
    pub(crate) effect: SceneEffect,
    pub(crate) document: SurfaceDocument,
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceRewriteEffect {
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) document: SurfaceDocument,
}
