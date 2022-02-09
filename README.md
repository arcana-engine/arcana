# arcana

[![crates](https://img.shields.io/crates/v/arcana.svg?style=for-the-badge&label=arcana)](https://crates.io/crates/arcana)
[![docs](https://img.shields.io/badge/docs.rs-arcana-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white)](https://docs.rs/arcana)
[![actions](https://img.shields.io/github/workflow/status/arcana-engine/arcana/badge/master?style=for-the-badge)](https://github.com/arcana-engine/arcana/actions?query=workflow%3ARust)
[![MIT/Apache](https://img.shields.io/badge/license-MIT%2FApache-blue.svg?style=for-the-badge)](COPYING)
![loc](https://img.shields.io/tokei/lines/github/arcana-engine/arcana?style=for-the-badge)


Arcana is a game engine built with focus on ease of use without compromising on level of control.

## Getting started

Starting writing a game is as simple as calling single function: `arcana::game2` or `arcana::game3`,\
depending on what number of dimensions new game needs.\
From there add systems, load prefabs or otherwise populate game world.

Then start writing prefab implementations and input controls, implement custom rendering logic when required.


## Examples

### Tanks
Playable example can be found in `examples/tanks`.

![Tanks example](images/tanks-latest.gif)

## License

Licensed under either of

* Apache License, Version 2.0, ([license/APACHE](license/APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([license/MIT](license/MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
