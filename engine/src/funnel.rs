use edict::world::World;

/// Funnel for input events.
pub trait Funnel<T> {
    /// Run event through this funnel.
    /// In response it may access resources and entities.
    fn filter(&mut self, world: &mut World, value: T) -> Option<T>;
}

impl<T, F> Funnel<T> for [F]
where
    F: Funnel<T>,
{
    fn filter(&mut self, world: &mut World, value: T) -> Option<T> {
        let mut value = value;
        for f in self {
            if let Some(filtered) = f.filter(world, value) {
                value = filtered;
            } else {
                return None;
            }
        }
        Some(value)
    }
}

impl<T, F, const N: usize> Funnel<T> for [F; N]
where
    F: Funnel<T>,
{
    fn filter(&mut self, world: &mut World, value: T) -> Option<T> {
        let mut value = value;
        for f in self {
            if let Some(filtered) = f.filter(world, value) {
                value = filtered;
            } else {
                return None;
            }
        }
        Some(value)
    }
}

impl<T> Funnel<T> for &mut dyn Funnel<T> {
    fn filter(&mut self, world: &mut World, value: T) -> Option<T> {
        Funnel::filter(&mut **self, world, value)
    }
}
