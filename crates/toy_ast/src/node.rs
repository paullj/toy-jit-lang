use toy_cst::{ResolvedNode, SyntaxKind};

pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: &ResolvedNode) -> Option<Self>;
    fn syntax(&self) -> &ResolvedNode;
}

macro_rules! ast_node {
    ($name:ident, $kind:pat) => {
        #[derive(Debug)]
        pub struct $name(ResolvedNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                matches!(kind, $kind)
            }
            fn cast(node: &ResolvedNode) -> Option<Self> {
                Self::can_cast(node.kind()).then(|| Self(node.clone()))
            }
            fn syntax(&self) -> &ResolvedNode {
                &self.0
            }
        }
    };
}

pub(crate) use ast_node;
