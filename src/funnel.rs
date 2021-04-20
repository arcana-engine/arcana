use {crate::resources::Res, hecs::World};

/// Funnel for input events.
pub trait Funnel<T> {
    /// Run event through this funnel.
    /// In response it may access resources and entities.
    fn filter(&mut self, res: &mut Res, world: &mut World, value: T) -> Option<T>;
}

/// Run value through the series of funnel.
pub fn run_funnel<T>(
    funnel: &mut [&mut dyn Funnel<T>],
    res: &mut Res,
    world: &mut World,
    value: T,
) -> Option<T> {
    let mut value = value;
    for f in funnel {
        if let Some(filtered) = f.filter(res, world, value) {
            value = filtered;
        } else {
            return None;
        }
    }
    Some(value)
}
