use std::marker::PhantomData;

use alkahest::Bytes;
use hecs::Component;
use scoped_arena::Scope;

#[cfg(feature = "client")]
use crate::CommandQueue;

#[cfg(feature = "client")]
use super::client;

// #[cfg(feature = "server")]
// use super::server;

pub struct ReplicaSerde<T>(PhantomData<fn() -> T>);

#[cfg(feature = "client")]
impl<T> client::ReplicaSetElem for ReplicaSerde<T>
where
    T: serde::de::DeserializeOwned + Component,
{
    type Component = T;
    type Replica = Bytes;

    #[inline(always)]
    fn build(unpacked: &[u8]) -> T {
        bincode::deserialize_from(unpacked).expect("Failed to deserialize item")
    }
}

// #[cfg(feature = "server")]
// impl<'a, T> server::ReplicaSetElem<'a> for ReplicaSerde<T>
// where
//     T: serde::Serialize + Component,
// {
//     type Component = T;
//     type Replica = Bytes;
//     type ReplicaPack = &'a [u8];

//     fn replicate(component: &T, scope: &'a Scope<'_>) -> &'a [u8] {
//         let mut out = Vec::new_in(scope);
//         bincode::serialize_into(&mut out, component).expect("Failed to serialize item");
//         out.leak()
//     }
// }

#[cfg(feature = "client")]
impl<'a, C> client::InputsReplicate<'a> for ReplicaSerde<C>
where
    C: Component + serde::Serialize,
{
    type Command = C;
    type Replica = Bytes;
    type ReplicaPack = &'a [u8];

    fn replicate(queue: &mut CommandQueue<C>, scope: &'a Scope<'_>) -> &'a [u8] {
        let commands = &*scope.to_scope_from_iter(queue.drain());

        let mut out = Vec::new_in(scope);
        bincode::serialize_into(&mut out, commands).expect("Failed to serialize item");
        out.leak()
    }
}
