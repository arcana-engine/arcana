use {
    super::Graphics,
    crate::{clocks::ClockIndex, resources::Res},
    bumpalo::Bump,
    hecs::World,
};

pub struct NodeContext<'a> {
    pub graphics: &'a mut Graphics,
    pub world: &'a mut World,
    pub res: &'a mut Res,
    pub bump: &'a Bump,
    pub clock: ClockIndex,
}

/// All rendering nodes implement this trait for ease of use.
/// Nodes need not be designed to be used generically.
pub trait Node {
    /// Resources consumed by the `Node`.
    /// This type contains all resources, images and buffers required
    /// for this Node to run.
    ///
    /// It is also useful to place "output" resources here,
    //// such as render attachments,
    /// so node would use them instead of creating own resources.
    type Input;

    /// Resources produced by the node. That can be put into following nodes.
    type Output;

    /// Run this node.
    fn run(&mut self, cx: NodeContext<'_>, input: Self::Input) -> eyre::Result<Self::Output>;
}
