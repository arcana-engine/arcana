#![feature(prelude_import)]
#![feature(allocator_api)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;

use arcana::{
    assets::{Asset, AssetId},
    edict::bundle::Bundle,
    na,
    physics2::{prelude::*, *},
    prelude::*,
    unfold::UnfoldResult,
};

pub struct Bullet;

pub struct BulletCollider(pub Collider);

impl Default for BulletCollider {
    fn default() -> Self {
        BulletCollider::new()
    }
}

impl BulletCollider {
    pub fn new() -> Self {
        BulletCollider(
            ColliderBuilder::ball(0.1)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .build(),
        )
    }
}

pub enum TankAnimTransitionRule {
    Moving,
    Idle,
    Broken,
    AnimationComplete,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for TankAnimTransitionRule {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&TankAnimTransitionRule::Moving,) => ::core::fmt::Formatter::write_str(f, "Moving"),
            (&TankAnimTransitionRule::Idle,) => ::core::fmt::Formatter::write_str(f, "Idle"),
            (&TankAnimTransitionRule::Broken,) => ::core::fmt::Formatter::write_str(f, "Broken"),
            (&TankAnimTransitionRule::AnimationComplete,) => {
                ::core::fmt::Formatter::write_str(f, "AnimationComplete")
            }
        }
    }
}

pub struct TankState {
    pub drive: i8,
    pub rotate: i8,
    pub fire: bool,
    pub alive: bool,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for TankState {
    #[inline]
    fn clone(&self) -> TankState {
        {
            let _: ::core::clone::AssertParamIsClone<i8>;
            let _: ::core::clone::AssertParamIsClone<i8>;
            let _: ::core::clone::AssertParamIsClone<bool>;
            let _: ::core::clone::AssertParamIsClone<bool>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for TankState {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for TankState {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            TankState {
                drive: ref __self_0_0,
                rotate: ref __self_0_1,
                fire: ref __self_0_2,
                alive: ref __self_0_3,
            } => {
                let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, "TankState");
                let _ =
                    ::core::fmt::DebugStruct::field(debug_trait_builder, "drive", &&(*__self_0_0));
                let _ =
                    ::core::fmt::DebugStruct::field(debug_trait_builder, "rotate", &&(*__self_0_1));
                let _ =
                    ::core::fmt::DebugStruct::field(debug_trait_builder, "fire", &&(*__self_0_2));
                let _ =
                    ::core::fmt::DebugStruct::field(debug_trait_builder, "alive", &&(*__self_0_3));
                ::core::fmt::DebugStruct::finish(debug_trait_builder)
            }
        }
    }
}
impl ::core::marker::StructuralPartialEq for TankState {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for TankState {
    #[inline]
    fn eq(&self, other: &TankState) -> bool {
        match *other {
            TankState {
                drive: ref __self_1_0,
                rotate: ref __self_1_1,
                fire: ref __self_1_2,
                alive: ref __self_1_3,
            } => match *self {
                TankState {
                    drive: ref __self_0_0,
                    rotate: ref __self_0_1,
                    fire: ref __self_0_2,
                    alive: ref __self_0_3,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &TankState) -> bool {
        match *other {
            TankState {
                drive: ref __self_1_0,
                rotate: ref __self_1_1,
                fire: ref __self_1_2,
                alive: ref __self_1_3,
            } => match *self {
                TankState {
                    drive: ref __self_0_0,
                    rotate: ref __self_0_1,
                    fire: ref __self_0_2,
                    alive: ref __self_0_3,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                }
            },
        }
    }
}
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl _serde::Serialize for TankState {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> _serde::__private::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = match _serde::Serializer::serialize_struct(
                __serializer,
                "TankState",
                false as usize + 1 + 1 + 1 + 1,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "drive",
                &self.drive,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "rotate",
                &self.rotate,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "fire",
                &self.fire,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "alive",
                &self.alive,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for TankState {
        fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __field3,
                __ignore,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(__Field::__field0),
                        1u64 => _serde::__private::Ok(__Field::__field1),
                        2u64 => _serde::__private::Ok(__Field::__field2),
                        3u64 => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "drive" => _serde::__private::Ok(__Field::__field0),
                        "rotate" => _serde::__private::Ok(__Field::__field1),
                        "fire" => _serde::__private::Ok(__Field::__field2),
                        "alive" => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"drive" => _serde::__private::Ok(__Field::__field0),
                        b"rotate" => _serde::__private::Ok(__Field::__field1),
                        b"fire" => _serde::__private::Ok(__Field::__field2),
                        b"alive" => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<TankState>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = TankState;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "struct TankState")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match match _serde::de::SeqAccess::next_element::<i8>(&mut __seq)
                    {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(_serde::de::Error::invalid_length(
                                0usize,
                                &"struct TankState with 4 elements",
                            ));
                        }
                    };
                    let __field1 = match match _serde::de::SeqAccess::next_element::<i8>(&mut __seq)
                    {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(_serde::de::Error::invalid_length(
                                1usize,
                                &"struct TankState with 4 elements",
                            ));
                        }
                    };
                    let __field2 =
                        match match _serde::de::SeqAccess::next_element::<bool>(&mut __seq) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    2usize,
                                    &"struct TankState with 4 elements",
                                ));
                            }
                        };
                    let __field3 =
                        match match _serde::de::SeqAccess::next_element::<bool>(&mut __seq) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    3usize,
                                    &"struct TankState with 4 elements",
                                ));
                            }
                        };
                    _serde::__private::Ok(TankState {
                        drive: __field0,
                        rotate: __field1,
                        fire: __field2,
                        alive: __field3,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private::Option<i8> = _serde::__private::None;
                    let mut __field1: _serde::__private::Option<i8> = _serde::__private::None;
                    let mut __field2: _serde::__private::Option<bool> = _serde::__private::None;
                    let mut __field3: _serde::__private::Option<bool> = _serde::__private::None;
                    while let _serde::__private::Some(__key) =
                        match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        }
                    {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private::Option::is_some(&__field0) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("drive"),
                                    );
                                }
                                __field0 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<i8>(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field1 => {
                                if _serde::__private::Option::is_some(&__field1) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "rotate",
                                        ),
                                    );
                                }
                                __field1 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<i8>(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field2 => {
                                if _serde::__private::Option::is_some(&__field2) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("fire"),
                                    );
                                }
                                __field2 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<bool>(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field3 => {
                                if _serde::__private::Option::is_some(&__field3) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("alive"),
                                    );
                                }
                                __field3 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<bool>(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            _ => {
                                let _ = match _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)
                                {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                };
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private::Some(__field0) => __field0,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("drive") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private::Some(__field1) => __field1,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("rotate") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::__private::Some(__field2) => __field2,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("fire") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field3 = match __field3 {
                        _serde::__private::Some(__field3) => __field3,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("alive") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    _serde::__private::Ok(TankState {
                        drive: __field0,
                        rotate: __field1,
                        fire: __field2,
                        alive: __field3,
                    })
                }
            }
            const FIELDS: &'static [&'static str] = &["drive", "rotate", "fire", "alive"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "TankState",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<TankState>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};

impl Default for TankState {
    fn default() -> Self {
        TankState::new()
    }
}

impl TankState {
    pub fn new() -> Self {
        TankState {
            drive: 0,
            rotate: 0,
            fire: false,
            alive: true,
        }
    }
}

pub enum TankCommand {
    Drive(i8),
    Rotate(i8),
    Fire,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for TankCommand {
    #[inline]
    fn clone(&self) -> TankCommand {
        {
            let _: ::core::clone::AssertParamIsClone<i8>;
            let _: ::core::clone::AssertParamIsClone<i8>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for TankCommand {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for TankCommand {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match (&*self,) {
            (&TankCommand::Drive(ref __self_0),) => {
                let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Drive");
                let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                ::core::fmt::DebugTuple::finish(debug_trait_builder)
            }
            (&TankCommand::Rotate(ref __self_0),) => {
                let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "Rotate");
                let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                ::core::fmt::DebugTuple::finish(debug_trait_builder)
            }
            (&TankCommand::Fire,) => ::core::fmt::Formatter::write_str(f, "Fire"),
        }
    }
}
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl _serde::Serialize for TankCommand {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> _serde::__private::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            match *self {
                TankCommand::Drive(ref __field0) => _serde::Serializer::serialize_newtype_variant(
                    __serializer,
                    "TankCommand",
                    0u32,
                    "Drive",
                    __field0,
                ),
                TankCommand::Rotate(ref __field0) => _serde::Serializer::serialize_newtype_variant(
                    __serializer,
                    "TankCommand",
                    1u32,
                    "Rotate",
                    __field0,
                ),
                TankCommand::Fire => _serde::Serializer::serialize_unit_variant(
                    __serializer,
                    "TankCommand",
                    2u32,
                    "Fire",
                ),
            }
        }
    }
};
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for TankCommand {
        fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
                __field1,
                __field2,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "variant identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(__Field::__field0),
                        1u64 => _serde::__private::Ok(__Field::__field1),
                        2u64 => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(__value),
                            &"variant index 0 <= i < 3",
                        )),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "Drive" => _serde::__private::Ok(__Field::__field0),
                        "Rotate" => _serde::__private::Ok(__Field::__field1),
                        "Fire" => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Err(_serde::de::Error::unknown_variant(
                            __value, VARIANTS,
                        )),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"Drive" => _serde::__private::Ok(__Field::__field0),
                        b"Rotate" => _serde::__private::Ok(__Field::__field1),
                        b"Fire" => _serde::__private::Ok(__Field::__field2),
                        _ => {
                            let __value = &_serde::__private::from_utf8_lossy(__value);
                            _serde::__private::Err(_serde::de::Error::unknown_variant(
                                __value, VARIANTS,
                            ))
                        }
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<TankCommand>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = TankCommand;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "enum TankCommand")
                }
                fn visit_enum<__A>(
                    self,
                    __data: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::EnumAccess<'de>,
                {
                    match match _serde::de::EnumAccess::variant(__data) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        (__Field::__field0, __variant) => _serde::__private::Result::map(
                            _serde::de::VariantAccess::newtype_variant::<i8>(__variant),
                            TankCommand::Drive,
                        ),
                        (__Field::__field1, __variant) => _serde::__private::Result::map(
                            _serde::de::VariantAccess::newtype_variant::<i8>(__variant),
                            TankCommand::Rotate,
                        ),
                        (__Field::__field2, __variant) => {
                            match _serde::de::VariantAccess::unit_variant(__variant) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            };
                            _serde::__private::Ok(TankCommand::Fire)
                        }
                    }
                }
            }
            const VARIANTS: &'static [&'static str] = &["Drive", "Rotate", "Fire"];
            _serde::Deserializer::deserialize_enum(
                __deserializer,
                "TankCommand",
                VARIANTS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<TankCommand>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};

pub struct BulletSystem;

impl System for BulletSystem {
    fn name(&self) -> &str {
        "BulletSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        let mut despawn = Vec::new_in(&*cx.scope);

        for (e, queue) in cx.world.query_mut::<&mut ContactQueue2>().with::<Bullet>() {
            if queue.drain_contacts_started().count() > 0 {
                despawn.push(e);
            }
            queue.drain_contacts_stopped();
        }

        for e in despawn {
            let _ = cx.world.despawn(&e);
        }
    }
}

#[asset(name = "tank")]
#[unfold(fn unfold_tank)]
pub struct Tank {
    pub size: na::Vector2<f32>,
    pub color: [f32; 3],

    pub sprite_sheet: AssetId,
}
impl ::arcana::unfold::Unfold for Tank {
    type UnfoldSystem = TankUnfoldSystem;
}
pub struct TankUnfoldSystem;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for TankUnfoldSystem {
    #[inline]
    fn clone(&self) -> TankUnfoldSystem {
        {
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for TankUnfoldSystem {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for TankUnfoldSystem {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            TankUnfoldSystem => ::core::fmt::Formatter::write_str(f, "TankUnfoldSystem"),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::default::Default for TankUnfoldSystem {
    #[inline]
    fn default() -> TankUnfoldSystem {
        TankUnfoldSystem {}
    }
}
struct TankUnfoldSpawned {
    size: na::Vector2<f32>,
    color: [f32; 3],
    sprite_sheet: AssetId,
}
impl ::arcana::system::System for TankUnfoldSystem {
    fn name(&self) -> &str {
        "Tank unfold system"
    }
    fn run(&mut self, cx: ::arcana::system::SystemContext<'_>) {
        use core::{
            any::type_name,
            borrow::Borrow,
            clone::Clone,
            iter::Iterator,
            option::Option::{self, Some, None},
        };
        use std::vec::Vec;
        use arcana::{
            assets::WithId,
            edict::{bundle::Bundle, entity::EntityId, world::World},
            unfold::UnfoldResult,
            resources::Res,
        };
        let cleanup_query = cx.world.query_mut::<&TankUnfoldSpawned>().without::<Tank>();
        let mut cleanup = Vec::new_in(&*cx.scope);
        cleanup.extend(cleanup_query.into_iter().map(|(e, _)| e));
        for e in cleanup {
            let _ = cx.world.remove::<TankUnfoldSpawned>(&e);
            fn cleanup<T: Bundle, I>(
                world: &mut World,
                entity: &EntityId,
                _: fn(&na::Vector2<f32>, &[f32; 3], &AssetId, &mut Res) -> UnfoldResult<T, I>,
            ) {
                let _ = world.remove_bundle::<T>(entity);
            }
            cleanup(cx.world, &e, unfold_tank);
        }
        let query = cx
            .world
            .query_mut::<(&Tank, Option<&mut TankUnfoldSpawned>)>();
        let mut unfolded_inserts = Vec::new_in(&*cx.scope);
        let mut inserts = Vec::new_in(&*cx.scope);
        let mut spawns = Vec::new_in(&*cx.scope);
        for (e, (value, unfolded)) in query {
            let mut unfolded_insert = None;
            let mut ready = true;
            let mut updated = false;
            let unfolded: &mut TankUnfoldSpawned = match unfolded {
                None => {
                    updated = true;
                    unfolded_insert.get_or_insert(TankUnfoldSpawned {
                        size: Clone::clone(&value.size),
                        color: Clone::clone(&value.color),
                        sprite_sheet: Clone::clone(&value.sprite_sheet),
                    })
                }
                Some(unfolded) => unfolded,
            };
            {
                if value.size != unfolded.size {
                    updated = true;
                }
            };
            {
                if value.color != unfolded.color {
                    updated = true;
                }
            };
            {
                if value.sprite_sheet != unfolded.sprite_sheet {
                    updated = true;
                }
            };
            if updated && ready {
                let UnfoldResult { insert, spawn } = (unfold_tank)(
                    &unfolded.size,
                    &unfolded.color,
                    &unfolded.sprite_sheet,
                    cx.res,
                );
                inserts.push((e, insert));
                if Iterator::size_hint(&spawn).1 != Some(0) {
                    spawns.push((e, spawn));
                }
            }
            if let Some(unfolded_insert) = unfolded_insert {
                unfolded_inserts.push((e, unfolded_insert));
            }
        }
        for (e, insert) in inserts {
            cx.world.try_insert_bundle(&e, insert).unwrap();
        }
        for (e, unfolded) in unfolded_inserts {
            cx.world.try_insert(&e, unfolded).unwrap();
        }
        for (e, spawn) in spawns {
            ::core::panicking::panic("not yet implemented")
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Tank {
    #[inline]
    fn clone(&self) -> Tank {
        {
            let _: ::core::clone::AssertParamIsClone<na::Vector2<f32>>;
            let _: ::core::clone::AssertParamIsClone<[f32; 3]>;
            let _: ::core::clone::AssertParamIsClone<AssetId>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Tank {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Tank {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Tank {
                size: ref __self_0_0,
                color: ref __self_0_1,
                sprite_sheet: ref __self_0_2,
            } => {
                let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, "Tank");
                let _ =
                    ::core::fmt::DebugStruct::field(debug_trait_builder, "size", &&(*__self_0_0));
                let _ =
                    ::core::fmt::DebugStruct::field(debug_trait_builder, "color", &&(*__self_0_1));
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "sprite_sheet",
                    &&(*__self_0_2),
                );
                ::core::fmt::DebugStruct::finish(debug_trait_builder)
            }
        }
    }
}
impl ::core::marker::StructuralPartialEq for Tank {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Tank {
    #[inline]
    fn eq(&self, other: &Tank) -> bool {
        match *other {
            Tank {
                size: ref __self_1_0,
                color: ref __self_1_1,
                sprite_sheet: ref __self_1_2,
            } => match *self {
                Tank {
                    size: ref __self_0_0,
                    color: ref __self_0_1,
                    sprite_sheet: ref __self_0_2,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Tank) -> bool {
        match *other {
            Tank {
                size: ref __self_1_0,
                color: ref __self_1_1,
                sprite_sheet: ref __self_1_2,
            } => match *self {
                Tank {
                    size: ref __self_0_0,
                    color: ref __self_0_1,
                    sprite_sheet: ref __self_0_2,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                }
            },
        }
    }
}
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl _serde::Serialize for Tank {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> _serde::__private::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = match _serde::Serializer::serialize_struct(
                __serializer,
                "Tank",
                false as usize + 1 + 1 + 1,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "size",
                &self.size,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "color",
                &self.color,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "sprite_sheet",
                &self.sprite_sheet,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for Tank {
        fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __ignore,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(__Field::__field0),
                        1u64 => _serde::__private::Ok(__Field::__field1),
                        2u64 => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "size" => _serde::__private::Ok(__Field::__field0),
                        "color" => _serde::__private::Ok(__Field::__field1),
                        "sprite_sheet" => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"size" => _serde::__private::Ok(__Field::__field0),
                        b"color" => _serde::__private::Ok(__Field::__field1),
                        b"sprite_sheet" => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<Tank>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = Tank;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "struct Tank")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match match _serde::de::SeqAccess::next_element::<na::Vector2<f32>>(
                        &mut __seq,
                    ) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(_serde::de::Error::invalid_length(
                                0usize,
                                &"struct Tank with 3 elements",
                            ));
                        }
                    };
                    let __field1 =
                        match match _serde::de::SeqAccess::next_element::<[f32; 3]>(&mut __seq) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    1usize,
                                    &"struct Tank with 3 elements",
                                ));
                            }
                        };
                    let __field2 =
                        match match _serde::de::SeqAccess::next_element::<AssetId>(&mut __seq) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    2usize,
                                    &"struct Tank with 3 elements",
                                ));
                            }
                        };
                    _serde::__private::Ok(Tank {
                        size: __field0,
                        color: __field1,
                        sprite_sheet: __field2,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private::Option<na::Vector2<f32>> =
                        _serde::__private::None;
                    let mut __field1: _serde::__private::Option<[f32; 3]> = _serde::__private::None;
                    let mut __field2: _serde::__private::Option<AssetId> = _serde::__private::None;
                    while let _serde::__private::Some(__key) =
                        match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        }
                    {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private::Option::is_some(&__field0) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("size"),
                                    );
                                }
                                __field0 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<na::Vector2<f32>>(
                                        &mut __map,
                                    ) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field1 => {
                                if _serde::__private::Option::is_some(&__field1) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("color"),
                                    );
                                }
                                __field1 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<[f32; 3]>(&mut __map)
                                    {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field2 => {
                                if _serde::__private::Option::is_some(&__field2) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "sprite_sheet",
                                        ),
                                    );
                                }
                                __field2 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<AssetId>(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            _ => {
                                let _ = match _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)
                                {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                };
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private::Some(__field0) => __field0,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("size") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private::Some(__field1) => __field1,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("color") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::__private::Some(__field2) => __field2,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("sprite_sheet") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    _serde::__private::Ok(Tank {
                        size: __field0,
                        color: __field1,
                        sprite_sheet: __field2,
                    })
                }
            }
            const FIELDS: &'static [&'static str] = &["size", "color", "sprite_sheet"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "Tank",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<Tank>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};
pub struct TankInfo {
    pub size: na::Vector2<f32>,
    pub color: [f32; 3],
    pub sprite_sheet: AssetId,
}
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl _serde::Serialize for TankInfo {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> _serde::__private::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = match _serde::Serializer::serialize_struct(
                __serializer,
                "TankInfo",
                false as usize + 1 + 1 + 1,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "size",
                &self.size,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "color",
                &self.color,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "sprite_sheet",
                &self.sprite_sheet,
            ) {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            };
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[allow(unused_macros)]
    macro_rules! try {
        ($__expr : expr) => {
            match $__expr {
                _serde::__private::Ok(__val) => __val,
                _serde::__private::Err(__err) => {
                    return _serde::__private::Err(__err);
                }
            }
        };
    }
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for TankInfo {
        fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __ignore,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(__Field::__field0),
                        1u64 => _serde::__private::Ok(__Field::__field1),
                        2u64 => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "size" => _serde::__private::Ok(__Field::__field0),
                        "color" => _serde::__private::Ok(__Field::__field1),
                        "sprite_sheet" => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"size" => _serde::__private::Ok(__Field::__field0),
                        b"color" => _serde::__private::Ok(__Field::__field1),
                        b"sprite_sheet" => _serde::__private::Ok(__Field::__field2),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<TankInfo>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = TankInfo;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "struct TankInfo")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match match _serde::de::SeqAccess::next_element::<na::Vector2<f32>>(
                        &mut __seq,
                    ) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(_serde::de::Error::invalid_length(
                                0usize,
                                &"struct TankInfo with 3 elements",
                            ));
                        }
                    };
                    let __field1 =
                        match match _serde::de::SeqAccess::next_element::<[f32; 3]>(&mut __seq) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    1usize,
                                    &"struct TankInfo with 3 elements",
                                ));
                            }
                        };
                    let __field2 =
                        match match _serde::de::SeqAccess::next_element::<AssetId>(&mut __seq) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        } {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(_serde::de::Error::invalid_length(
                                    2usize,
                                    &"struct TankInfo with 3 elements",
                                ));
                            }
                        };
                    _serde::__private::Ok(TankInfo {
                        size: __field0,
                        color: __field1,
                        sprite_sheet: __field2,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private::Option<na::Vector2<f32>> =
                        _serde::__private::None;
                    let mut __field1: _serde::__private::Option<[f32; 3]> = _serde::__private::None;
                    let mut __field2: _serde::__private::Option<AssetId> = _serde::__private::None;
                    while let _serde::__private::Some(__key) =
                        match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                            _serde::__private::Ok(__val) => __val,
                            _serde::__private::Err(__err) => {
                                return _serde::__private::Err(__err);
                            }
                        }
                    {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private::Option::is_some(&__field0) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("size"),
                                    );
                                }
                                __field0 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<na::Vector2<f32>>(
                                        &mut __map,
                                    ) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field1 => {
                                if _serde::__private::Option::is_some(&__field1) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("color"),
                                    );
                                }
                                __field1 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<[f32; 3]>(&mut __map)
                                    {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field2 => {
                                if _serde::__private::Option::is_some(&__field2) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "sprite_sheet",
                                        ),
                                    );
                                }
                                __field2 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<AssetId>(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            _ => {
                                let _ = match _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)
                                {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                };
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private::Some(__field0) => __field0,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("size") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private::Some(__field1) => __field1,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("color") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::__private::Some(__field2) => __field2,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("sprite_sheet") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    _serde::__private::Ok(TankInfo {
                        size: __field0,
                        color: __field1,
                        sprite_sheet: __field2,
                    })
                }
            }
            const FIELDS: &'static [&'static str] = &["size", "color", "sprite_sheet"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "TankInfo",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<TankInfo>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};
impl ::goods::TrivialAsset for Tank {
    type Error = ::goods::DecodeError;
    fn name() -> &'static str {
        "tank"
    }
    fn decode(bytes: ::std::boxed::Box<[u8]>) -> Result<Self, ::goods::DecodeError> {
        use {
            std::result::Result::{Ok, Err},
            goods::serde_json::error::Category,
        };
        #[doc = r" Zero-length is definitely bincode."]
        let decoded: TankInfo =
            if bytes.is_empty() {
                    match ::goods::bincode::deserialize(&*bytes) {
                        Ok(value) => value,
                        Err(err) => return Err(::goods::DecodeError::Bincode(err)),
                    }
                } else {
                   match ::goods::serde_json::from_slice(&*bytes) {
                       Ok(value) => value,
                       Err(err) =>
                           match err.classify() {
                               Category::Syntax => {
                                   match ::goods::bincode::deserialize(&*bytes) {
                                       Ok(value) => value,
                                       Err(err) => return Err(::goods::DecodeError::Bincode(err)),
                                   }
                               }
                               _ => return Err(::goods::DecodeError::Json(err)),
                           },
                   }
               };
        Ok(Tank {
            size: decoded.size,
            color: decoded.color,
            sprite_sheet: decoded.sprite_sheet,
        })
    }
}
impl ::goods::AssetField<::goods::Container> for Tank {
    type BuildError = ::std::convert::Infallible;
    type DecodeError = ::std::convert::Infallible;
    type Info = TankInfo;
    type Decoded = Self;
    type Fut = ::std::future::Ready<Result<Self, ::std::convert::Infallible>>;
    fn decode(info: TankInfo, _: &::goods::Loader) -> Self::Fut {
        use std::{future::ready, result::Result::Ok};
        let decoded = info;
        ready(Ok(Tank {
            size: decoded.size,
            color: decoded.color,
            sprite_sheet: decoded.sprite_sheet,
        }))
    }
}
impl<BuilderGenericParameter> ::goods::AssetFieldBuild<::goods::Container, BuilderGenericParameter>
    for Tank
{
    fn build(
        decoded: Self,
        builder: &mut BuilderGenericParameter,
    ) -> Result<Self, ::std::convert::Infallible> {
        ::std::result::Result::Ok(decoded)
    }
}
#[allow(unused_variables)]
fn unfold_tank(
    size: &na::Vector2<f32>,
    color: &[f32; 3],
    #[cfg(not(feature = "graphics"))] _sprite_sheet: &AssetId,
    res: &mut Res,
) -> UnfoldResult<impl Bundle> {
    let hs = size / 2.0;
    let physics = res.with(PhysicsData2::new);
    let body = physics.bodies.insert(
        RigidBodyBuilder::new_dynamic()
            .linear_damping(0.3)
            .angular_damping(0.3)
            .build(),
    );
    physics.colliders.insert_with_parent(
        ColliderBuilder::cuboid(hs.x * 0.625, hs.y * 0.6875)
            .active_events(ActiveEvents::CONTACT_EVENTS)
            .build(),
        body,
        &mut physics.bodies,
    );
    UnfoldResult::with_bundle((body, ContactQueue2::new()))
}
