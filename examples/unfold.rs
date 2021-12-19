#![feature(allocator_api)]

use arcana::prelude::*;
use goods::*;

fn main() {}

#[derive(Clone, Asset)]
#[asset(name = "A")]
struct A;

#[derive(Clone, Asset)]
#[asset(name = "B")]
struct B;

#[derive(Unfold)]
pub struct Foo {
    #[unfold(asset: A)]
    a: AssetId,

    #[unfold(asset)]
    b: TypedAssetId<B>,
}

#[derive(Unfold)]
#[unfold(fn unfold_bar)]
pub struct Bar {
    #[unfold(asset: A)]
    a: AssetId,

    #[unfold(asset)]
    b: TypedAssetId<B>,
}

fn unfold_bar() {}
