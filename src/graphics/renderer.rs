pub mod basic;
pub mod sprite;

use {
    super::Graphics,
    crate::{bitset::BoxedBitSet, clocks::ClockIndex, resources::Res, viewport::Viewport},
    bumpalo::Bump,
    hecs::World,
    std::{
        collections::hash_map::{Entry, HashMap},
        hash::Hash,
    },
};

pub struct RendererContext<'a> {
    pub res: &'a mut Res,
    pub world: &'a mut World,
    pub graphics: &'a mut Graphics,
    pub bump: &'a Bump,
    pub clock: ClockIndex,
}

impl RendererContext<'_> {
    fn reborrow(&mut self) -> RendererContext<'_> {
        RendererContext {
            res: &mut *self.res,
            world: &mut *self.world,
            graphics: &mut *self.graphics,
            bump: &*self.bump,
            clock: self.clock,
        }
    }
}

/// Abstract rendering system.
pub trait Renderer: 'static {
    fn new(graphics: &mut Graphics) -> eyre::Result<Self>
    where
        Self: Sized;

    /// Render into specified viewports.
    fn render(
        &mut self,
        cx: RendererContext<'_>,
        viewports: &mut [&mut Viewport],
    ) -> eyre::Result<()>;
}

#[inline(always)]
fn mat4_na_to_sierra(m: na::Matrix4<f32>) -> sierra::mat4<f32> {
    let array: [[f32; 4]; 4] = m.into();
    sierra::mat4::from(array)
}

#[inline(always)]
fn mat3_na_to_sierra(m: na::Matrix3<f32>) -> sierra::mat3<f32> {
    let array: [[f32; 3]; 3] = m.into();
    sierra::mat3::from(array)
}

struct SparseDescriptors<T> {
    resources: HashMap<T, u32>,
    bitset: BoxedBitSet,
    next: u32,
}

impl<T> SparseDescriptors<T>
where
    T: Hash + Eq,
{
    fn new() -> Self {
        SparseDescriptors {
            resources: HashMap::new(),
            bitset: BoxedBitSet::new(),
            next: 0,
        }
    }

    fn index(&mut self, resource: T) -> (u32, bool) {
        match self.resources.entry(resource) {
            Entry::Occupied(entry) => (*entry.get(), false),
            Entry::Vacant(entry) => {
                if let Some(index) = self.bitset.find_set() {
                    self.bitset.unset(index);
                    (*entry.insert(index as u32), true)
                } else {
                    self.next += 1;
                    (*entry.insert(self.next - 1), true)
                }
            }
        }
    }
}
