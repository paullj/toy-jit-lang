use syntax::{SyntaxKind, SyntaxNode};

/// Trait for AST nodes wrapping a `SyntaxNode`.
pub trait AstNode: Sized {
    /// Returns true if this node kind can be cast to this type.
    fn can_cast(kind: SyntaxKind) -> bool;

    /// Attempt to cast a syntax node. Returns `None` if wrong kind.
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// Returns the underlying syntax node.
    fn syntax(&self) -> &SyntaxNode;
}

/// Defines an AST node struct with `AstNode` impl.
macro_rules! ast_node {
    ($name:ident, $kind:pat) => {
        #[derive(Debug)]
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                matches!(kind, $kind)
            }
            fn cast(node: SyntaxNode) -> Option<Self> {
                Self::can_cast(node.kind()).then_some(Self(node))
            }
            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

pub(crate) use ast_node;
