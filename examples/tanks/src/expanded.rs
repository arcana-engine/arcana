#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2018::*;
#[macro_use]
extern crate std;
use tank::TankSystem;

use {arcana::{assets::tiles::TileMap, camera::Camera2, game2, na, Global2,
              Local2, Physics2, SystemContext}, std::time::Duration};

mod tank {





    // game.scheduler.add_system(Physics2::new());







    use {arcana::{assets::ImageAsset, bumpalo::collections::Vec as BVec,
                  event::{DeviceEvent, ElementState, KeyboardInput,
                          VirtualKeyCode},
                  graphics::{Graphics, ImageView, Material, Rect, Sprite,
                             Texture}, hecs::{Entity, World}, ContactQueue2,
                  ControlResult, Global2, InputController, PhysicsData2,
                  Prefab, Res, System, SystemContext},
         futures::future::BoxFuture,
         goods::{Asset, AssetHandle, AssetResult, Loader},
         ordered_float::OrderedFloat,
         rapier2d::{dynamics::{RigidBodyBuilder, RigidBodyHandle},
                    geometry::{Collider, ColliderBuilder}},
         std::{future::ready, time::Duration}, uuid::Uuid};
    pub struct Frame {
        pub rect: Rect,
        pub duration_us: u64,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Frame {
        #[inline]
        fn clone(&self) -> Frame {
            match *self {
                Frame { rect: ref __self_0_0, duration_us: ref __self_0_1 } =>
                Frame{rect: ::core::clone::Clone::clone(&(*__self_0_0)),
                      duration_us:
                          ::core::clone::Clone::clone(&(*__self_0_1)),},
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Frame {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Frame { rect: ref __self_0_0, duration_us: ref __self_0_1 } =>
                {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Frame");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "rect",
                                                        &&(*__self_0_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "duration_us",
                                                        &&(*__self_0_1));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () =
        {
            #[allow(unused_extern_crates, clippy :: useless_attribute)]
            extern crate serde as _serde;
            #[allow(unused_macros)]
            macro_rules! try {
                ($ __expr : expr) =>
                {
                    match $ __expr
                    {
                        _serde :: __private :: Ok(__val) => __val, _serde ::
                        __private :: Err(__err) =>
                        { return _serde :: __private :: Err(__err) ; }
                    }
                }
            }
            #[automatically_derived]
            impl <'de> _serde::Deserialize<'de> for Frame {
                fn deserialize<__D>(__deserializer: __D)
                 -> _serde::__private::Result<Self, __D::Error> where
                 __D: _serde::Deserializer<'de> {
                    #[allow(non_camel_case_types)]
                    enum __Field { __field0, __field1, __ignore, }
                    struct __FieldVisitor;
                    impl <'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(&self,
                                     __formatter:
                                         &mut _serde::__private::Formatter)
                         -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter,
                                                                    "field identifier")
                        }
                        fn visit_u64<__E>(self, __value: u64)
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                0u64 =>
                                _serde::__private::Ok(__Field::__field0),
                                1u64 =>
                                _serde::__private::Ok(__Field::__field1),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_str<__E>(self, __value: &str)
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                "rect" =>
                                _serde::__private::Ok(__Field::__field0),
                                "duration_us" =>
                                _serde::__private::Ok(__Field::__field1),
                                _ => {
                                    _serde::__private::Ok(__Field::__ignore)
                                }
                            }
                        }
                        fn visit_bytes<__E>(self, __value: &[u8])
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                b"rect" =>
                                _serde::__private::Ok(__Field::__field0),
                                b"duration_us" =>
                                _serde::__private::Ok(__Field::__field1),
                                _ => {
                                    _serde::__private::Ok(__Field::__ignore)
                                }
                            }
                        }
                    }
                    impl <'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(__deserializer: __D)
                         -> _serde::__private::Result<Self, __D::Error> where
                         __D: _serde::Deserializer<'de> {
                            _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                         __FieldVisitor)
                        }
                    }
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<Frame>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = Frame;
                        fn expecting(&self,
                                     __formatter:
                                         &mut _serde::__private::Formatter)
                         -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter,
                                                                    "struct Frame")
                        }
                        #[inline]
                        fn visit_seq<__A>(self, mut __seq: __A)
                         -> _serde::__private::Result<Self::Value, __A::Error>
                         where __A: _serde::de::SeqAccess<'de> {
                            let __field0 =
                                match match _serde::de::SeqAccess::next_element::<Rect>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                        &"struct Frame with 2 elements"));
                                    }
                                };
                            let __field1 =
                                match match _serde::de::SeqAccess::next_element::<u64>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                        &"struct Frame with 2 elements"));
                                    }
                                };
                            _serde::__private::Ok(Frame{rect: __field0,
                                                        duration_us:
                                                            __field1,})
                        }
                        #[inline]
                        fn visit_map<__A>(self, mut __map: __A)
                         -> _serde::__private::Result<Self::Value, __A::Error>
                         where __A: _serde::de::MapAccess<'de> {
                            let mut __field0:
                                    _serde::__private::Option<Rect> =
                                _serde::__private::None;
                            let mut __field1: _serde::__private::Option<u64> =
                                _serde::__private::None;
                            while let _serde::__private::Some(__key) =
                                      match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                match __key {
                                    __Field::__field0 => {
                                        if _serde::__private::Option::is_some(&__field0)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("rect"));
                                        }
                                        __field0 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<Rect>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    __Field::__field1 => {
                                        if _serde::__private::Option::is_some(&__field1)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("duration_us"));
                                        }
                                        __field1 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<u64>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    _ => {
                                        let _ =
                                            match _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(&mut __map)
                                                {
                                                _serde::__private::Ok(__val)
                                                => __val,
                                                _serde::__private::Err(__err)
                                                => {
                                                    return _serde::__private::Err(__err);
                                                }
                                            };
                                    }
                                }
                            }
                            let __field0 =
                                match __field0 {
                                    _serde::__private::Some(__field0) =>
                                    __field0,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("rect")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            let __field1 =
                                match __field1 {
                                    _serde::__private::Some(__field1) =>
                                    __field1,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("duration_us")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            _serde::__private::Ok(Frame{rect: __field0,
                                                        duration_us:
                                                            __field1,})
                        }
                    }
                    const FIELDS: &'static [&'static str] =
                        &["rect", "duration_us"];
                    _serde::Deserializer::deserialize_struct(__deserializer,
                                                             "Frame", FIELDS,
                                                             __Visitor{marker:
                                                                           _serde::__private::PhantomData::<Frame>,
                                                                       lifetime:
                                                                           _serde::__private::PhantomData,})
                }
            }
        };
    pub struct SpriteSheet {
        pub frames: Vec<Frame>,
        pub animations: Vec<Animation>,
        #[external]
        pub image: Texture,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for SpriteSheet {
        #[inline]
        fn clone(&self) -> SpriteSheet {
            match *self {
                SpriteSheet {
                frames: ref __self_0_0,
                animations: ref __self_0_1,
                image: ref __self_0_2 } =>
                SpriteSheet{frames:
                                ::core::clone::Clone::clone(&(*__self_0_0)),
                            animations:
                                ::core::clone::Clone::clone(&(*__self_0_1)),
                            image:
                                ::core::clone::Clone::clone(&(*__self_0_2)),},
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for SpriteSheet {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                SpriteSheet {
                frames: ref __self_0_0,
                animations: ref __self_0_1,
                image: ref __self_0_2 } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f,
                                                                  "SpriteSheet");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "frames",
                                                        &&(*__self_0_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "animations",
                                                        &&(*__self_0_1));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "image",
                                                        &&(*__self_0_2));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    struct SpriteSheetInfo {
        frames: Vec<Frame>,
        animations: Vec<Animation>,
        image: <Texture as ::goods::AssetField<::goods::External>>::Info,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () =
        {
            #[allow(unused_extern_crates, clippy :: useless_attribute)]
            extern crate serde as _serde;
            #[allow(unused_macros)]
            macro_rules! try {
                ($ __expr : expr) =>
                {
                    match $ __expr
                    {
                        _serde :: __private :: Ok(__val) => __val, _serde ::
                        __private :: Err(__err) =>
                        { return _serde :: __private :: Err(__err) ; }
                    }
                }
            }
            #[automatically_derived]
            impl <'de> _serde::Deserialize<'de> for SpriteSheetInfo {
                fn deserialize<__D>(__deserializer: __D)
                 -> _serde::__private::Result<Self, __D::Error> where
                 __D: _serde::Deserializer<'de> {
                    #[allow(non_camel_case_types)]
                    enum __Field { __field0, __field1, __field2, __ignore, }
                    struct __FieldVisitor;
                    impl <'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(&self,
                                     __formatter:
                                         &mut _serde::__private::Formatter)
                         -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter,
                                                                    "field identifier")
                        }
                        fn visit_u64<__E>(self, __value: u64)
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                0u64 =>
                                _serde::__private::Ok(__Field::__field0),
                                1u64 =>
                                _serde::__private::Ok(__Field::__field1),
                                2u64 =>
                                _serde::__private::Ok(__Field::__field2),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_str<__E>(self, __value: &str)
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                "frames" =>
                                _serde::__private::Ok(__Field::__field0),
                                "animations" =>
                                _serde::__private::Ok(__Field::__field1),
                                "image" =>
                                _serde::__private::Ok(__Field::__field2),
                                _ => {
                                    _serde::__private::Ok(__Field::__ignore)
                                }
                            }
                        }
                        fn visit_bytes<__E>(self, __value: &[u8])
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                b"frames" =>
                                _serde::__private::Ok(__Field::__field0),
                                b"animations" =>
                                _serde::__private::Ok(__Field::__field1),
                                b"image" =>
                                _serde::__private::Ok(__Field::__field2),
                                _ => {
                                    _serde::__private::Ok(__Field::__ignore)
                                }
                            }
                        }
                    }
                    impl <'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(__deserializer: __D)
                         -> _serde::__private::Result<Self, __D::Error> where
                         __D: _serde::Deserializer<'de> {
                            _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                         __FieldVisitor)
                        }
                    }
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<SpriteSheetInfo>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = SpriteSheetInfo;
                        fn expecting(&self,
                                     __formatter:
                                         &mut _serde::__private::Formatter)
                         -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter,
                                                                    "struct SpriteSheetInfo")
                        }
                        #[inline]
                        fn visit_seq<__A>(self, mut __seq: __A)
                         -> _serde::__private::Result<Self::Value, __A::Error>
                         where __A: _serde::de::SeqAccess<'de> {
                            let __field0 =
                                match match _serde::de::SeqAccess::next_element::<Vec<Frame>>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                        &"struct SpriteSheetInfo with 3 elements"));
                                    }
                                };
                            let __field1 =
                                match match _serde::de::SeqAccess::next_element::<Vec<Animation>>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                        &"struct SpriteSheetInfo with 3 elements"));
                                    }
                                };
                            let __field2 =
                                match match _serde::de::SeqAccess::next_element::<<Texture
                                                                                  as
                                                                                  ::goods::AssetField<::goods::External>>::Info>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(2usize,
                                                                                                        &"struct SpriteSheetInfo with 3 elements"));
                                    }
                                };
                            _serde::__private::Ok(SpriteSheetInfo{frames:
                                                                      __field0,
                                                                  animations:
                                                                      __field1,
                                                                  image:
                                                                      __field2,})
                        }
                        #[inline]
                        fn visit_map<__A>(self, mut __map: __A)
                         -> _serde::__private::Result<Self::Value, __A::Error>
                         where __A: _serde::de::MapAccess<'de> {
                            let mut __field0:
                                    _serde::__private::Option<Vec<Frame>> =
                                _serde::__private::None;
                            let mut __field1:
                                    _serde::__private::Option<Vec<Animation>> =
                                _serde::__private::None;
                            let mut __field2:
                                    _serde::__private::Option<<Texture as
                                                              ::goods::AssetField<::goods::External>>::Info> =
                                _serde::__private::None;
                            while let _serde::__private::Some(__key) =
                                      match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                match __key {
                                    __Field::__field0 => {
                                        if _serde::__private::Option::is_some(&__field0)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("frames"));
                                        }
                                        __field0 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<Vec<Frame>>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    __Field::__field1 => {
                                        if _serde::__private::Option::is_some(&__field1)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("animations"));
                                        }
                                        __field1 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<Vec<Animation>>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    __Field::__field2 => {
                                        if _serde::__private::Option::is_some(&__field2)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("image"));
                                        }
                                        __field2 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<<Texture
                                                                                                              as
                                                                                                              ::goods::AssetField<::goods::External>>::Info>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    _ => {
                                        let _ =
                                            match _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(&mut __map)
                                                {
                                                _serde::__private::Ok(__val)
                                                => __val,
                                                _serde::__private::Err(__err)
                                                => {
                                                    return _serde::__private::Err(__err);
                                                }
                                            };
                                    }
                                }
                            }
                            let __field0 =
                                match __field0 {
                                    _serde::__private::Some(__field0) =>
                                    __field0,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("frames")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            let __field1 =
                                match __field1 {
                                    _serde::__private::Some(__field1) =>
                                    __field1,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("animations")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            let __field2 =
                                match __field2 {
                                    _serde::__private::Some(__field2) =>
                                    __field2,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("image")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            _serde::__private::Ok(SpriteSheetInfo{frames:
                                                                      __field0,
                                                                  animations:
                                                                      __field1,
                                                                  image:
                                                                      __field2,})
                        }
                    }
                    const FIELDS: &'static [&'static str] =
                        &["frames", "animations", "image"];
                    _serde::Deserializer::deserialize_struct(__deserializer,
                                                             "SpriteSheetInfo",
                                                             FIELDS,
                                                             __Visitor{marker:
                                                                           _serde::__private::PhantomData::<SpriteSheetInfo>,
                                                                       lifetime:
                                                                           _serde::__private::PhantomData,})
                }
            }
        };
    struct SpriteSheetfutures {
        frames: Vec<Frame>,
        animations: Vec<Animation>,
        image: <Texture as ::goods::AssetField<::goods::External>>::Fut,
    }
    pub struct SpriteSheetDecoded {
        frames: Vec<Frame>,
        animations: Vec<Animation>,
        image: <Texture as ::goods::AssetField<::goods::External>>::Decoded,
    }
    pub enum SpriteSheetDecodeError {

        #[error("Failed to deserialize asset info from json")]
        Json(
             #[source]
             ::goods::serde_json::Error),

        #[error("Failed to deserialize asset info from bincode")]
        Bincode(
                #[source]
                ::goods::bincode::Error),

        #[error("Failed to decode asset field \'image\'")]
        ImageError {
            source: <Texture as
                    ::goods::AssetField<::goods::External>>::DecodeError,
        },
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for SpriteSheetDecodeError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&SpriteSheetDecodeError::Json(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Json");
                    let _ =
                        ::core::fmt::DebugTuple::field(debug_trait_builder,
                                                       &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&SpriteSheetDecodeError::Bincode(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f,
                                                                 "Bincode");
                    let _ =
                        ::core::fmt::DebugTuple::field(debug_trait_builder,
                                                       &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&SpriteSheetDecodeError::ImageError { source: ref __self_0
                 },) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f,
                                                                  "ImageError");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "source",
                                                        &&(*__self_0));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::error::Error for SpriteSheetDecodeError {
        fn source(&self)
         -> std::option::Option<&(dyn std::error::Error + 'static)> {
            use thiserror::private::AsDynError;

            #[allow(deprecated)]
            match self {
                SpriteSheetDecodeError::Json { 0: source, .. } =>
                std::option::Option::Some(source.as_dyn_error()),
                SpriteSheetDecodeError::Bincode { 0: source, .. } =>
                std::option::Option::Some(source.as_dyn_error()),
                SpriteSheetDecodeError::ImageError { source: source, .. } =>
                std::option::Option::Some(source.as_dyn_error()),
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::fmt::Display for SpriteSheetDecodeError {
        fn fmt(&self, __formatter: &mut std::fmt::Formatter)
         -> std::fmt::Result {

            #[allow(unused_variables, deprecated, clippy ::
                    used_underscore_binding)]
            match self {
                SpriteSheetDecodeError::Json(_0) =>
                __formatter.write_fmt(::core::fmt::Arguments::new_v1(&["Failed to deserialize asset info from json"],
                                                                     &match ()
                                                                          {
                                                                          ()
                                                                          =>
                                                                          [],
                                                                      })),
                SpriteSheetDecodeError::Bincode(_0) =>
                __formatter.write_fmt(::core::fmt::Arguments::new_v1(&["Failed to deserialize asset info from bincode"],
                                                                     &match ()
                                                                          {
                                                                          ()
                                                                          =>
                                                                          [],
                                                                      })),
                SpriteSheetDecodeError::ImageError { source } =>
                __formatter.write_fmt(::core::fmt::Arguments::new_v1(&["Failed to decode asset field \'image\'"],
                                                                     &match ()
                                                                          {
                                                                          ()
                                                                          =>
                                                                          [],
                                                                      })),
            }
        }
    }
    pub enum SpriteSheetBuildError {

        #[error("Failed to build asset field \'image\'")]
        ImageError {
            source: <Texture as
                    ::goods::AssetField<::goods::External>>::BuildError,
        },
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for SpriteSheetBuildError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&SpriteSheetBuildError::ImageError { source: ref __self_0 },)
                => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f,
                                                                  "ImageError");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "source",
                                                        &&(*__self_0));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::error::Error for SpriteSheetBuildError {
        fn source(&self)
         -> std::option::Option<&(dyn std::error::Error + 'static)> {
            use thiserror::private::AsDynError;

            #[allow(deprecated)]
            match self {
                SpriteSheetBuildError::ImageError { source: source, .. } =>
                std::option::Option::Some(source.as_dyn_error()),
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::fmt::Display for SpriteSheetBuildError {
        fn fmt(&self, __formatter: &mut std::fmt::Formatter)
         -> std::fmt::Result {

            #[allow(unused_variables, deprecated, clippy ::
                    used_underscore_binding)]
            match self {
                SpriteSheetBuildError::ImageError { source } =>
                __formatter.write_fmt(::core::fmt::Arguments::new_v1(&["Failed to build asset field \'image\'"],
                                                                     &match ()
                                                                          {
                                                                          ()
                                                                          =>
                                                                          [],
                                                                      })),
            }
        }
    }
    impl ::goods::Asset for SpriteSheet {
        type BuildError = SpriteSheetBuildError;
        type DecodeError = SpriteSheetDecodeError;
        type Decoded = SpriteSheetDecoded;
        type Fut =
         ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output =
                                                                     Result<SpriteSheetDecoded,
                                                                            SpriteSheetDecodeError>> +
                                           Send>>;
        fn decode(bytes: ::std::boxed::Box<[u8]>, loader: &::goods::Loader)
         -> Self::Fut {
            use {::std::{boxed::Box, result::Result::{self, Ok, Err}},
                 ::goods::serde_json::error::Category};
            let result: Result<SpriteSheetInfo, SpriteSheetDecodeError> =
                if bytes.is_empty() {
                    match ::goods::bincode::deserialize(&*bytes) {
                        Ok(value) => Ok(value),
                        Err(err) => Err(SpriteSheetDecodeError::Bincode(err)),
                    }
                } else {
                    match ::goods::serde_json::from_slice(&*bytes) {
                        Ok(value) => Ok(value),
                        Err(err) =>
                        match err.classify() {
                            Category::Syntax => {
                                match ::goods::bincode::deserialize(&*bytes) {
                                    Ok(value) => Ok(value),
                                    Err(err) =>
                                    Err(SpriteSheetDecodeError::Bincode(err)),
                                }
                            }
                            _ => { Err(SpriteSheetDecodeError::Json(err)) }
                        },
                    }
                };
            match result {
                Ok(info) => {
                    let futures =
                        SpriteSheetfutures{frames: info.frames,
                                           animations: info.animations,
                                           image:
                                               <Texture as
                                                   ::goods::AssetField<::goods::External>>::decode(info.image,
                                                                                                   loader),};
                    Box::pin(async move
                                 {
                                     Ok(SpriteSheetDecoded{frames:
                                                               futures.frames,
                                                           animations:
                                                               futures.animations,
                                                           image:
                                                               futures.image.await.map_err(|err|
                                                                                               SpriteSheetDecodeError::ImageError{source:
                                                                                                                                      err,})?,})
                                 })
                }
                Err(err) => Box::pin(async move  { Err(err) }),
            }
        }
    }
    impl <BuilderGenericParameter>
     ::goods::AssetBuild<BuilderGenericParameter> for SpriteSheet where
     Texture: ::goods::AssetFieldBuild<::goods::External,
                                       BuilderGenericParameter> {
        fn build(decoded: SpriteSheetDecoded,
                 builder: &mut BuilderGenericParameter)
         -> Result<Self, SpriteSheetBuildError> {
            ::std::result::Result::Ok(SpriteSheet{frames: decoded.frames,
                                                  animations:
                                                      decoded.animations,
                                                  image:
                                                      <Texture as
                                                          ::std::convert::From<Texture>>::from(<Texture
                                                                                                   as
                                                                                                   ::goods::AssetFieldBuild<::goods::External,
                                                                                                                            BuilderGenericParameter>>::build(decoded.image,
                                                                                                                                                             builder).map_err(|err|
                                                                                                                                                                                  SpriteSheetBuildError::ImageError{source:
                                                                                                                                                                                                                        err,})?),})
        }
    }
    pub struct Animation {
        pub name: Box<str>,
        pub from: usize,
        pub to: usize,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Animation {
        #[inline]
        fn clone(&self) -> Animation {
            match *self {
                Animation {
                name: ref __self_0_0, from: ref __self_0_1, to: ref __self_0_2
                } =>
                Animation{name: ::core::clone::Clone::clone(&(*__self_0_0)),
                          from: ::core::clone::Clone::clone(&(*__self_0_1)),
                          to: ::core::clone::Clone::clone(&(*__self_0_2)),},
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Animation {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Animation {
                name: ref __self_0_0, from: ref __self_0_1, to: ref __self_0_2
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f,
                                                                  "Animation");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "name",
                                                        &&(*__self_0_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "from",
                                                        &&(*__self_0_1));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "to",
                                                        &&(*__self_0_2));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    pub struct AnimationInfo {
        name: Box<str>,
        from: usize,
        to: usize,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () =
        {
            #[allow(unused_extern_crates, clippy :: useless_attribute)]
            extern crate serde as _serde;
            #[allow(unused_macros)]
            macro_rules! try {
                ($ __expr : expr) =>
                {
                    match $ __expr
                    {
                        _serde :: __private :: Ok(__val) => __val, _serde ::
                        __private :: Err(__err) =>
                        { return _serde :: __private :: Err(__err) ; }
                    }
                }
            }
            #[automatically_derived]
            impl <'de> _serde::Deserialize<'de> for AnimationInfo {
                fn deserialize<__D>(__deserializer: __D)
                 -> _serde::__private::Result<Self, __D::Error> where
                 __D: _serde::Deserializer<'de> {
                    #[allow(non_camel_case_types)]
                    enum __Field { __field0, __field1, __field2, __ignore, }
                    struct __FieldVisitor;
                    impl <'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(&self,
                                     __formatter:
                                         &mut _serde::__private::Formatter)
                         -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter,
                                                                    "field identifier")
                        }
                        fn visit_u64<__E>(self, __value: u64)
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                0u64 =>
                                _serde::__private::Ok(__Field::__field0),
                                1u64 =>
                                _serde::__private::Ok(__Field::__field1),
                                2u64 =>
                                _serde::__private::Ok(__Field::__field2),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_str<__E>(self, __value: &str)
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                "name" =>
                                _serde::__private::Ok(__Field::__field0),
                                "from" =>
                                _serde::__private::Ok(__Field::__field1),
                                "to" =>
                                _serde::__private::Ok(__Field::__field2),
                                _ => {
                                    _serde::__private::Ok(__Field::__ignore)
                                }
                            }
                        }
                        fn visit_bytes<__E>(self, __value: &[u8])
                         -> _serde::__private::Result<Self::Value, __E> where
                         __E: _serde::de::Error {
                            match __value {
                                b"name" =>
                                _serde::__private::Ok(__Field::__field0),
                                b"from" =>
                                _serde::__private::Ok(__Field::__field1),
                                b"to" =>
                                _serde::__private::Ok(__Field::__field2),
                                _ => {
                                    _serde::__private::Ok(__Field::__ignore)
                                }
                            }
                        }
                    }
                    impl <'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(__deserializer: __D)
                         -> _serde::__private::Result<Self, __D::Error> where
                         __D: _serde::Deserializer<'de> {
                            _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                         __FieldVisitor)
                        }
                    }
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<AnimationInfo>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl <'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = AnimationInfo;
                        fn expecting(&self,
                                     __formatter:
                                         &mut _serde::__private::Formatter)
                         -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter,
                                                                    "struct AnimationInfo")
                        }
                        #[inline]
                        fn visit_seq<__A>(self, mut __seq: __A)
                         -> _serde::__private::Result<Self::Value, __A::Error>
                         where __A: _serde::de::SeqAccess<'de> {
                            let __field0 =
                                match match _serde::de::SeqAccess::next_element::<Box<str>>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                        &"struct AnimationInfo with 3 elements"));
                                    }
                                };
                            let __field1 =
                                match match _serde::de::SeqAccess::next_element::<usize>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                        &"struct AnimationInfo with 3 elements"));
                                    }
                                };
                            let __field2 =
                                match match _serde::de::SeqAccess::next_element::<usize>(&mut __seq)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                    _serde::__private::Some(__value) =>
                                    __value,
                                    _serde::__private::None => {
                                        return _serde::__private::Err(_serde::de::Error::invalid_length(2usize,
                                                                                                        &"struct AnimationInfo with 3 elements"));
                                    }
                                };
                            _serde::__private::Ok(AnimationInfo{name:
                                                                    __field0,
                                                                from:
                                                                    __field1,
                                                                to:
                                                                    __field2,})
                        }
                        #[inline]
                        fn visit_map<__A>(self, mut __map: __A)
                         -> _serde::__private::Result<Self::Value, __A::Error>
                         where __A: _serde::de::MapAccess<'de> {
                            let mut __field0:
                                    _serde::__private::Option<Box<str>> =
                                _serde::__private::None;
                            let mut __field1:
                                    _serde::__private::Option<usize> =
                                _serde::__private::None;
                            let mut __field2:
                                    _serde::__private::Option<usize> =
                                _serde::__private::None;
                            while let _serde::__private::Some(__key) =
                                      match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                          {
                                          _serde::__private::Ok(__val) =>
                                          __val,
                                          _serde::__private::Err(__err) => {
                                              return _serde::__private::Err(__err);
                                          }
                                      } {
                                match __key {
                                    __Field::__field0 => {
                                        if _serde::__private::Option::is_some(&__field0)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("name"));
                                        }
                                        __field0 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<Box<str>>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    __Field::__field1 => {
                                        if _serde::__private::Option::is_some(&__field1)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("from"));
                                        }
                                        __field1 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<usize>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    __Field::__field2 => {
                                        if _serde::__private::Option::is_some(&__field2)
                                           {
                                            return _serde::__private::Err(<__A::Error
                                                                              as
                                                                              _serde::de::Error>::duplicate_field("to"));
                                        }
                                        __field2 =
                                            _serde::__private::Some(match _serde::de::MapAccess::next_value::<usize>(&mut __map)
                                                                        {
                                                                        _serde::__private::Ok(__val)
                                                                        =>
                                                                        __val,
                                                                        _serde::__private::Err(__err)
                                                                        => {
                                                                            return _serde::__private::Err(__err);
                                                                        }
                                                                    });
                                    }
                                    _ => {
                                        let _ =
                                            match _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(&mut __map)
                                                {
                                                _serde::__private::Ok(__val)
                                                => __val,
                                                _serde::__private::Err(__err)
                                                => {
                                                    return _serde::__private::Err(__err);
                                                }
                                            };
                                    }
                                }
                            }
                            let __field0 =
                                match __field0 {
                                    _serde::__private::Some(__field0) =>
                                    __field0,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("name")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            let __field1 =
                                match __field1 {
                                    _serde::__private::Some(__field1) =>
                                    __field1,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("from")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            let __field2 =
                                match __field2 {
                                    _serde::__private::Some(__field2) =>
                                    __field2,
                                    _serde::__private::None =>
                                    match _serde::__private::de::missing_field("to")
                                        {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                };
                            _serde::__private::Ok(AnimationInfo{name:
                                                                    __field0,
                                                                from:
                                                                    __field1,
                                                                to:
                                                                    __field2,})
                        }
                    }
                    const FIELDS: &'static [&'static str] =
                        &["name", "from", "to"];
                    _serde::Deserializer::deserialize_struct(__deserializer,
                                                             "AnimationInfo",
                                                             FIELDS,
                                                             __Visitor{marker:
                                                                           _serde::__private::PhantomData::<AnimationInfo>,
                                                                       lifetime:
                                                                           _serde::__private::PhantomData,})
                }
            }
        };
    pub enum AnimationDecodeError {

        #[error("Failed to deserialize asset info from json")]
        Json(
             #[source]
             ::goods::serde_json::Error),

        #[error("Failed to deserialize asset info from bincode")]
        Bincode(
                #[source]
                ::goods::bincode::Error),
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for AnimationDecodeError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&AnimationDecodeError::Json(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "Json");
                    let _ =
                        ::core::fmt::DebugTuple::field(debug_trait_builder,
                                                       &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&AnimationDecodeError::Bincode(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f,
                                                                 "Bincode");
                    let _ =
                        ::core::fmt::DebugTuple::field(debug_trait_builder,
                                                       &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::error::Error for AnimationDecodeError {
        fn source(&self)
         -> std::option::Option<&(dyn std::error::Error + 'static)> {
            use thiserror::private::AsDynError;

            #[allow(deprecated)]
            match self {
                AnimationDecodeError::Json { 0: source, .. } =>
                std::option::Option::Some(source.as_dyn_error()),
                AnimationDecodeError::Bincode { 0: source, .. } =>
                std::option::Option::Some(source.as_dyn_error()),
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::fmt::Display for AnimationDecodeError {
        fn fmt(&self, __formatter: &mut std::fmt::Formatter)
         -> std::fmt::Result {

            #[allow(unused_variables, deprecated, clippy ::
                    used_underscore_binding)]
            match self {
                AnimationDecodeError::Json(_0) =>
                __formatter.write_fmt(::core::fmt::Arguments::new_v1(&["Failed to deserialize asset info from json"],
                                                                     &match ()
                                                                          {
                                                                          ()
                                                                          =>
                                                                          [],
                                                                      })),
                AnimationDecodeError::Bincode(_0) =>
                __formatter.write_fmt(::core::fmt::Arguments::new_v1(&["Failed to deserialize asset info from bincode"],
                                                                     &match ()
                                                                          {
                                                                          ()
                                                                          =>
                                                                          [],
                                                                      })),
            }
        }
    }
    pub type AnimationBuildError = ::std::convert::Infallible;
    impl ::goods::Asset for Animation {
        type BuildError = AnimationBuildError;
        type DecodeError = AnimationDecodeError;
        type Decoded = AnimationInfo;
        type Fut =
         ::std::future::Ready<Result<AnimationInfo, AnimationDecodeError>>;
        fn decode(bytes: ::std::boxed::Box<[u8]>, _loader: &::goods::Loader)
         -> Self::Fut {
            use {::std::result::Result::{Ok, Err},
                 ::goods::serde_json::error::Category};
            #[doc = r" Zero-length is definitely bincode."]
            let result =
                if bytes.is_empty() {
                    match ::goods::bincode::deserialize(&*bytes) {
                        Ok(value) => Ok(value),
                        Err(err) => Err(AnimationDecodeError::Bincode(err)),
                    }
                } else {
                    match ::goods::serde_json::from_slice(&*bytes) {
                        Ok(value) => Ok(value),
                        Err(err) =>
                        match err.classify() {
                            Category::Syntax => {
                                match ::goods::bincode::deserialize(&*bytes) {
                                    Ok(value) => Ok(value),
                                    Err(err) =>
                                    Err(AnimationDecodeError::Bincode(err)),
                                }
                            }
                            _ => { Err(AnimationDecodeError::Json(err)) }
                        },
                    }
                };
            ::std::future::ready(result)
        }
    }
    impl <BuilderGenericParameter>
     ::goods::AssetBuild<BuilderGenericParameter> for Animation {
        fn build(decoded: AnimationInfo,
                 _builder: &mut BuilderGenericParameter)
         -> Result<Self, AnimationBuildError> {
            Ok(Animation{name: decoded.name,
                         from: decoded.from,
                         to: decoded.to,})
        }
    }
    mod serde_impls {
        use {super::*, serde::de::*};
        pub struct FrameDe {
            pub frame: Rect,
            pub duration: f32,
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes,
                unused_qualifications)]
        const _: () =
            {
                #[allow(unused_extern_crates, clippy :: useless_attribute)]
                extern crate serde as _serde;
                #[allow(unused_macros)]
                macro_rules! try {
                    ($ __expr : expr) =>
                    {
                        match $ __expr
                        {
                            _serde :: __private :: Ok(__val) => __val, _serde
                            :: __private :: Err(__err) =>
                            { return _serde :: __private :: Err(__err) ; }
                        }
                    }
                }
                #[automatically_derived]
                impl <'de> _serde::Deserialize<'de> for FrameDe {
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::__private::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        #[allow(non_camel_case_types)]
                        enum __Field { __field0, __field1, __ignore, }
                        struct __FieldVisitor;
                        impl <'de> _serde::de::Visitor<'de> for __FieldVisitor
                         {
                            type Value = __Field;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "field identifier")
                            }
                            fn visit_u64<__E>(self, __value: u64)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    0u64 =>
                                    _serde::__private::Ok(__Field::__field0),
                                    1u64 =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ =>
                                    _serde::__private::Ok(__Field::__ignore),
                                }
                            }
                            fn visit_str<__E>(self, __value: &str)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    "frame" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    "duration" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                            fn visit_bytes<__E>(self, __value: &[u8])
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    b"frame" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    b"duration" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                        }
                        impl <'de> _serde::Deserialize<'de> for __Field {
                            #[inline]
                            fn deserialize<__D>(__deserializer: __D)
                             -> _serde::__private::Result<Self, __D::Error>
                             where __D: _serde::Deserializer<'de> {
                                _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                             __FieldVisitor)
                            }
                        }
                        struct __Visitor<'de> {
                            marker: _serde::__private::PhantomData<FrameDe>,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl <'de> _serde::de::Visitor<'de> for __Visitor<'de>
                         {
                            type Value = FrameDe;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "struct FrameDe")
                            }
                            #[inline]
                            fn visit_seq<__A>(self, mut __seq: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::SeqAccess<'de> {
                                let __field0 =
                                    match match _serde::de::SeqAccess::next_element::<Rect>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                            &"struct FrameDe with 2 elements"));
                                        }
                                    };
                                let __field1 =
                                    match match _serde::de::SeqAccess::next_element::<f32>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                            &"struct FrameDe with 2 elements"));
                                        }
                                    };
                                _serde::__private::Ok(FrameDe{frame: __field0,
                                                              duration:
                                                                  __field1,})
                            }
                            #[inline]
                            fn visit_map<__A>(self, mut __map: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::MapAccess<'de> {
                                let mut __field0:
                                        _serde::__private::Option<Rect> =
                                    _serde::__private::None;
                                let mut __field1:
                                        _serde::__private::Option<f32> =
                                    _serde::__private::None;
                                while let _serde::__private::Some(__key) =
                                          match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                    match __key {
                                        __Field::__field0 => {
                                            if _serde::__private::Option::is_some(&__field0)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("frame"));
                                            }
                                            __field0 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<Rect>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        __Field::__field1 => {
                                            if _serde::__private::Option::is_some(&__field1)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("duration"));
                                            }
                                            __field1 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<f32>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        _ => {
                                            let _ =
                                                match _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(&mut __map)
                                                    {
                                                    _serde::__private::Ok(__val)
                                                    => __val,
                                                    _serde::__private::Err(__err)
                                                    => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                };
                                        }
                                    }
                                }
                                let __field0 =
                                    match __field0 {
                                        _serde::__private::Some(__field0) =>
                                        __field0,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("frame")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                let __field1 =
                                    match __field1 {
                                        _serde::__private::Some(__field1) =>
                                        __field1,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("duration")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                _serde::__private::Ok(FrameDe{frame: __field0,
                                                              duration:
                                                                  __field1,})
                            }
                        }
                        const FIELDS: &'static [&'static str] =
                            &["frame", "duration"];
                        _serde::Deserializer::deserialize_struct(__deserializer,
                                                                 "FrameDe",
                                                                 FIELDS,
                                                                 __Visitor{marker:
                                                                               _serde::__private::PhantomData::<FrameDe>,
                                                                           lifetime:
                                                                               _serde::__private::PhantomData,})
                    }
                }
            };
        struct SpriteSheetDe {
            frames: Vec<FrameDe>,
            meta: SpriteSheetMeta,
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes,
                unused_qualifications)]
        const _: () =
            {
                #[allow(unused_extern_crates, clippy :: useless_attribute)]
                extern crate serde as _serde;
                #[allow(unused_macros)]
                macro_rules! try {
                    ($ __expr : expr) =>
                    {
                        match $ __expr
                        {
                            _serde :: __private :: Ok(__val) => __val, _serde
                            :: __private :: Err(__err) =>
                            { return _serde :: __private :: Err(__err) ; }
                        }
                    }
                }
                #[automatically_derived]
                impl <'de> _serde::Deserialize<'de> for SpriteSheetDe {
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::__private::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        #[allow(non_camel_case_types)]
                        enum __Field { __field0, __field1, __ignore, }
                        struct __FieldVisitor;
                        impl <'de> _serde::de::Visitor<'de> for __FieldVisitor
                         {
                            type Value = __Field;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "field identifier")
                            }
                            fn visit_u64<__E>(self, __value: u64)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    0u64 =>
                                    _serde::__private::Ok(__Field::__field0),
                                    1u64 =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ =>
                                    _serde::__private::Ok(__Field::__ignore),
                                }
                            }
                            fn visit_str<__E>(self, __value: &str)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    "frames" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    "meta" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                            fn visit_bytes<__E>(self, __value: &[u8])
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    b"frames" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    b"meta" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                        }
                        impl <'de> _serde::Deserialize<'de> for __Field {
                            #[inline]
                            fn deserialize<__D>(__deserializer: __D)
                             -> _serde::__private::Result<Self, __D::Error>
                             where __D: _serde::Deserializer<'de> {
                                _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                             __FieldVisitor)
                            }
                        }
                        struct __Visitor<'de> {
                            marker: _serde::__private::PhantomData<SpriteSheetDe>,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl <'de> _serde::de::Visitor<'de> for __Visitor<'de>
                         {
                            type Value = SpriteSheetDe;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "struct SpriteSheetDe")
                            }
                            #[inline]
                            fn visit_seq<__A>(self, mut __seq: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::SeqAccess<'de> {
                                let __field0 =
                                    match match _serde::de::SeqAccess::next_element::<Vec<FrameDe>>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                            &"struct SpriteSheetDe with 2 elements"));
                                        }
                                    };
                                let __field1 =
                                    match match _serde::de::SeqAccess::next_element::<SpriteSheetMeta>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                            &"struct SpriteSheetDe with 2 elements"));
                                        }
                                    };
                                _serde::__private::Ok(SpriteSheetDe{frames:
                                                                        __field0,
                                                                    meta:
                                                                        __field1,})
                            }
                            #[inline]
                            fn visit_map<__A>(self, mut __map: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::MapAccess<'de> {
                                let mut __field0:
                                        _serde::__private::Option<Vec<FrameDe>> =
                                    _serde::__private::None;
                                let mut __field1:
                                        _serde::__private::Option<SpriteSheetMeta> =
                                    _serde::__private::None;
                                while let _serde::__private::Some(__key) =
                                          match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                    match __key {
                                        __Field::__field0 => {
                                            if _serde::__private::Option::is_some(&__field0)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("frames"));
                                            }
                                            __field0 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<Vec<FrameDe>>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        __Field::__field1 => {
                                            if _serde::__private::Option::is_some(&__field1)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("meta"));
                                            }
                                            __field1 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<SpriteSheetMeta>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        _ => {
                                            let _ =
                                                match _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(&mut __map)
                                                    {
                                                    _serde::__private::Ok(__val)
                                                    => __val,
                                                    _serde::__private::Err(__err)
                                                    => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                };
                                        }
                                    }
                                }
                                let __field0 =
                                    match __field0 {
                                        _serde::__private::Some(__field0) =>
                                        __field0,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("frames")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                let __field1 =
                                    match __field1 {
                                        _serde::__private::Some(__field1) =>
                                        __field1,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("meta")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                _serde::__private::Ok(SpriteSheetDe{frames:
                                                                        __field0,
                                                                    meta:
                                                                        __field1,})
                            }
                        }
                        const FIELDS: &'static [&'static str] =
                            &["frames", "meta"];
                        _serde::Deserializer::deserialize_struct(__deserializer,
                                                                 "SpriteSheetDe",
                                                                 FIELDS,
                                                                 __Visitor{marker:
                                                                               _serde::__private::PhantomData::<SpriteSheetDe>,
                                                                           lifetime:
                                                                               _serde::__private::PhantomData,})
                    }
                }
            };
        struct Size {
            w: f32,
            h: f32,
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes,
                unused_qualifications)]
        const _: () =
            {
                #[allow(unused_extern_crates, clippy :: useless_attribute)]
                extern crate serde as _serde;
                #[allow(unused_macros)]
                macro_rules! try {
                    ($ __expr : expr) =>
                    {
                        match $ __expr
                        {
                            _serde :: __private :: Ok(__val) => __val, _serde
                            :: __private :: Err(__err) =>
                            { return _serde :: __private :: Err(__err) ; }
                        }
                    }
                }
                #[automatically_derived]
                impl <'de> _serde::Deserialize<'de> for Size {
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::__private::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        #[allow(non_camel_case_types)]
                        enum __Field { __field0, __field1, __ignore, }
                        struct __FieldVisitor;
                        impl <'de> _serde::de::Visitor<'de> for __FieldVisitor
                         {
                            type Value = __Field;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "field identifier")
                            }
                            fn visit_u64<__E>(self, __value: u64)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    0u64 =>
                                    _serde::__private::Ok(__Field::__field0),
                                    1u64 =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ =>
                                    _serde::__private::Ok(__Field::__ignore),
                                }
                            }
                            fn visit_str<__E>(self, __value: &str)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    "w" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    "h" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                            fn visit_bytes<__E>(self, __value: &[u8])
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    b"w" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    b"h" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                        }
                        impl <'de> _serde::Deserialize<'de> for __Field {
                            #[inline]
                            fn deserialize<__D>(__deserializer: __D)
                             -> _serde::__private::Result<Self, __D::Error>
                             where __D: _serde::Deserializer<'de> {
                                _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                             __FieldVisitor)
                            }
                        }
                        struct __Visitor<'de> {
                            marker: _serde::__private::PhantomData<Size>,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl <'de> _serde::de::Visitor<'de> for __Visitor<'de>
                         {
                            type Value = Size;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "struct Size")
                            }
                            #[inline]
                            fn visit_seq<__A>(self, mut __seq: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::SeqAccess<'de> {
                                let __field0 =
                                    match match _serde::de::SeqAccess::next_element::<f32>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                            &"struct Size with 2 elements"));
                                        }
                                    };
                                let __field1 =
                                    match match _serde::de::SeqAccess::next_element::<f32>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                            &"struct Size with 2 elements"));
                                        }
                                    };
                                _serde::__private::Ok(Size{w: __field0,
                                                           h: __field1,})
                            }
                            #[inline]
                            fn visit_map<__A>(self, mut __map: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::MapAccess<'de> {
                                let mut __field0:
                                        _serde::__private::Option<f32> =
                                    _serde::__private::None;
                                let mut __field1:
                                        _serde::__private::Option<f32> =
                                    _serde::__private::None;
                                while let _serde::__private::Some(__key) =
                                          match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                    match __key {
                                        __Field::__field0 => {
                                            if _serde::__private::Option::is_some(&__field0)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("w"));
                                            }
                                            __field0 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<f32>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        __Field::__field1 => {
                                            if _serde::__private::Option::is_some(&__field1)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("h"));
                                            }
                                            __field1 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<f32>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        _ => {
                                            let _ =
                                                match _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(&mut __map)
                                                    {
                                                    _serde::__private::Ok(__val)
                                                    => __val,
                                                    _serde::__private::Err(__err)
                                                    => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                };
                                        }
                                    }
                                }
                                let __field0 =
                                    match __field0 {
                                        _serde::__private::Some(__field0) =>
                                        __field0,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("w")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                let __field1 =
                                    match __field1 {
                                        _serde::__private::Some(__field1) =>
                                        __field1,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("h")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                _serde::__private::Ok(Size{w: __field0,
                                                           h: __field1,})
                            }
                        }
                        const FIELDS: &'static [&'static str] = &["w", "h"];
                        _serde::Deserializer::deserialize_struct(__deserializer,
                                                                 "Size",
                                                                 FIELDS,
                                                                 __Visitor{marker:
                                                                               _serde::__private::PhantomData::<Size>,
                                                                           lifetime:
                                                                               _serde::__private::PhantomData,})
                    }
                }
            };
        struct SpriteSheetMeta {
            image: Box<str>,
            #[serde(rename = "frameTags")]
            animations: Vec<Animation>,
            size: Size,
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes,
                unused_qualifications)]
        const _: () =
            {
                #[allow(unused_extern_crates, clippy :: useless_attribute)]
                extern crate serde as _serde;
                #[allow(unused_macros)]
                macro_rules! try {
                    ($ __expr : expr) =>
                    {
                        match $ __expr
                        {
                            _serde :: __private :: Ok(__val) => __val, _serde
                            :: __private :: Err(__err) =>
                            { return _serde :: __private :: Err(__err) ; }
                        }
                    }
                }
                #[automatically_derived]
                impl <'de> _serde::Deserialize<'de> for SpriteSheetMeta {
                    fn deserialize<__D>(__deserializer: __D)
                     -> _serde::__private::Result<Self, __D::Error> where
                     __D: _serde::Deserializer<'de> {
                        #[allow(non_camel_case_types)]
                        enum __Field {
                            __field0,
                            __field1,
                            __field2,
                            __ignore,
                        }
                        struct __FieldVisitor;
                        impl <'de> _serde::de::Visitor<'de> for __FieldVisitor
                         {
                            type Value = __Field;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "field identifier")
                            }
                            fn visit_u64<__E>(self, __value: u64)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    0u64 =>
                                    _serde::__private::Ok(__Field::__field0),
                                    1u64 =>
                                    _serde::__private::Ok(__Field::__field1),
                                    2u64 =>
                                    _serde::__private::Ok(__Field::__field2),
                                    _ =>
                                    _serde::__private::Ok(__Field::__ignore),
                                }
                            }
                            fn visit_str<__E>(self, __value: &str)
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    "image" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    "frameTags" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    "size" =>
                                    _serde::__private::Ok(__Field::__field2),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                            fn visit_bytes<__E>(self, __value: &[u8])
                             -> _serde::__private::Result<Self::Value, __E>
                             where __E: _serde::de::Error {
                                match __value {
                                    b"image" =>
                                    _serde::__private::Ok(__Field::__field0),
                                    b"frameTags" =>
                                    _serde::__private::Ok(__Field::__field1),
                                    b"size" =>
                                    _serde::__private::Ok(__Field::__field2),
                                    _ => {
                                        _serde::__private::Ok(__Field::__ignore)
                                    }
                                }
                            }
                        }
                        impl <'de> _serde::Deserialize<'de> for __Field {
                            #[inline]
                            fn deserialize<__D>(__deserializer: __D)
                             -> _serde::__private::Result<Self, __D::Error>
                             where __D: _serde::Deserializer<'de> {
                                _serde::Deserializer::deserialize_identifier(__deserializer,
                                                                             __FieldVisitor)
                            }
                        }
                        struct __Visitor<'de> {
                            marker: _serde::__private::PhantomData<SpriteSheetMeta>,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl <'de> _serde::de::Visitor<'de> for __Visitor<'de>
                         {
                            type Value = SpriteSheetMeta;
                            fn expecting(&self,
                                         __formatter:
                                             &mut _serde::__private::Formatter)
                             -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(__formatter,
                                                                        "struct SpriteSheetMeta")
                            }
                            #[inline]
                            fn visit_seq<__A>(self, mut __seq: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::SeqAccess<'de> {
                                let __field0 =
                                    match match _serde::de::SeqAccess::next_element::<Box<str>>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(0usize,
                                                                                                            &"struct SpriteSheetMeta with 3 elements"));
                                        }
                                    };
                                let __field1 =
                                    match match _serde::de::SeqAccess::next_element::<Vec<Animation>>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(1usize,
                                                                                                            &"struct SpriteSheetMeta with 3 elements"));
                                        }
                                    };
                                let __field2 =
                                    match match _serde::de::SeqAccess::next_element::<Size>(&mut __seq)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                        _serde::__private::Some(__value) =>
                                        __value,
                                        _serde::__private::None => {
                                            return _serde::__private::Err(_serde::de::Error::invalid_length(2usize,
                                                                                                            &"struct SpriteSheetMeta with 3 elements"));
                                        }
                                    };
                                _serde::__private::Ok(SpriteSheetMeta{image:
                                                                          __field0,
                                                                      animations:
                                                                          __field1,
                                                                      size:
                                                                          __field2,})
                            }
                            #[inline]
                            fn visit_map<__A>(self, mut __map: __A)
                             ->
                                 _serde::__private::Result<Self::Value,
                                                           __A::Error> where
                             __A: _serde::de::MapAccess<'de> {
                                let mut __field0:
                                        _serde::__private::Option<Box<str>> =
                                    _serde::__private::None;
                                let mut __field1:
                                        _serde::__private::Option<Vec<Animation>> =
                                    _serde::__private::None;
                                let mut __field2:
                                        _serde::__private::Option<Size> =
                                    _serde::__private::None;
                                while let _serde::__private::Some(__key) =
                                          match _serde::de::MapAccess::next_key::<__Field>(&mut __map)
                                              {
                                              _serde::__private::Ok(__val) =>
                                              __val,
                                              _serde::__private::Err(__err) =>
                                              {
                                                  return _serde::__private::Err(__err);
                                              }
                                          } {
                                    match __key {
                                        __Field::__field0 => {
                                            if _serde::__private::Option::is_some(&__field0)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("image"));
                                            }
                                            __field0 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<Box<str>>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        __Field::__field1 => {
                                            if _serde::__private::Option::is_some(&__field1)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("frameTags"));
                                            }
                                            __field1 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<Vec<Animation>>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        __Field::__field2 => {
                                            if _serde::__private::Option::is_some(&__field2)
                                               {
                                                return _serde::__private::Err(<__A::Error
                                                                                  as
                                                                                  _serde::de::Error>::duplicate_field("size"));
                                            }
                                            __field2 =
                                                _serde::__private::Some(match _serde::de::MapAccess::next_value::<Size>(&mut __map)
                                                                            {
                                                                            _serde::__private::Ok(__val)
                                                                            =>
                                                                            __val,
                                                                            _serde::__private::Err(__err)
                                                                            =>
                                                                            {
                                                                                return _serde::__private::Err(__err);
                                                                            }
                                                                        });
                                        }
                                        _ => {
                                            let _ =
                                                match _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(&mut __map)
                                                    {
                                                    _serde::__private::Ok(__val)
                                                    => __val,
                                                    _serde::__private::Err(__err)
                                                    => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                };
                                        }
                                    }
                                }
                                let __field0 =
                                    match __field0 {
                                        _serde::__private::Some(__field0) =>
                                        __field0,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("image")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                let __field1 =
                                    match __field1 {
                                        _serde::__private::Some(__field1) =>
                                        __field1,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("frameTags")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                let __field2 =
                                    match __field2 {
                                        _serde::__private::Some(__field2) =>
                                        __field2,
                                        _serde::__private::None =>
                                        match _serde::__private::de::missing_field("size")
                                            {
                                            _serde::__private::Ok(__val) =>
                                            __val,
                                            _serde::__private::Err(__err) => {
                                                return _serde::__private::Err(__err);
                                            }
                                        },
                                    };
                                _serde::__private::Ok(SpriteSheetMeta{image:
                                                                          __field0,
                                                                      animations:
                                                                          __field1,
                                                                      size:
                                                                          __field2,})
                            }
                        }
                        const FIELDS: &'static [&'static str] =
                            &["image", "frameTags", "size"];
                        _serde::Deserializer::deserialize_struct(__deserializer,
                                                                 "SpriteSheetMeta",
                                                                 FIELDS,
                                                                 __Visitor{marker:
                                                                               _serde::__private::PhantomData::<SpriteSheetMeta>,
                                                                           lifetime:
                                                                               _serde::__private::PhantomData,})
                    }
                }
            };
        impl <'de> Deserialize<'de> for SpriteSheetInfo {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
             D: Deserializer<'de> {
                let sheet = SpriteSheetDe::deserialize(deserializer)?;
                let width = sheet.meta.size.w;
                let height = sheet.meta.size.h;
                Ok(SpriteSheetInfo{frames:
                                       sheet.frames.into_iter().map(|f|
                                                                        Frame{rect:
                                                                                  Rect{left:
                                                                                           f.frame.left
                                                                                               /
                                                                                               width,
                                                                                       right:
                                                                                           f.frame.right
                                                                                               /
                                                                                               width,
                                                                                       top:
                                                                                           f.frame.top
                                                                                               /
                                                                                               height,
                                                                                       bottom:
                                                                                           f.frame.bottom
                                                                                               /
                                                                                               height,},
                                                                              duration_us:
                                                                                  (f.duration
                                                                                       *
                                                                                       1000.0)
                                                                                      as
                                                                                      u64,}).collect(),
                                   animations: sheet.meta.animations,
                                   image: sheet.meta.image,})
            }
        }
    }
    impl Asset for SpriteSheet {
        type Error = assets::Error;
        type Decoded = SpriteSheetDecoded;
        type Builder = Graphics;
        type Fut =
         BoxFuture<'static, Result<SpriteSheetDecoded, serde_json::Error>>;
        fn decode(bytes: Box<[u8]>, loader: Loader) -> Self::Fut {
            match serde_json::from_slice::<SpriteSheetInfo>(&*bytes) {
                Ok(info) =>
                Box::pin(async move
                             {
                                 Ok(SpriteSheetDecoded{frames: info.frames,
                                                       animations:
                                                           info.animations,
                                                       image:
                                                           loader.load(&info.image).await,})
                             }),
                Err(err) => Box::pin(ready(Err(err))),
            }
        }
        fn build(mut decoded: SpriteSheetDecoded, graphics: &mut Graphics)
         -> Result<Self, assets::Error> {
            let image = decoded.image.get_existing(graphics)?;
            Ok(SpriteSheet{frames: decoded.frames,
                           animations: decoded.animations,
                           image: image.image.clone(),})
        }
    }
    pub struct Bullet;
    struct BulletCollider(Collider);
    impl BulletCollider {
        fn new() -> Self {
            BulletCollider(ColliderBuilder::ball(0.1).build())
        }
    }
    pub struct Tank {
        sprite_sheet: Box<str>,
        size: na::Vector2<f32>,
        color: [f32; 3],
    }
    impl Tank {
        pub fn new(sprite_sheet: Box<str>, size: na::Vector2<f32>,
                   color: [f32; 3]) -> Self {
            Tank{sprite_sheet, size, color,}
        }
    }
    impl Prefab for Tank {
        type Loaded = AssetResult<SpriteSheet>;
        type Fut = AssetHandle<SpriteSheet>;
        fn load(&self, loader: &Loader) -> Self::Fut {
            loader.load(&self.sprite_sheet)
        }
        fn spawn(mut sprite_sheet: AssetResult<SpriteSheet>, res: &mut Res,
                 world: &mut World, graphics: &mut Graphics, entity: Entity)
         -> eyre::Result<()> {
            let tank = world.get_mut::<Self>(entity)?;
            let size = tank.size;
            let color = tank.color;
            drop(tank);
            let sprite_sheet = sprite_sheet.get_existing(graphics)?;
            let sampler = graphics.create_sampler(Default::default())?;
            let physics = res.with(PhysicsData2::new);
            let hs = size * 0.5;
            let body =
                physics.bodies.insert(RigidBodyBuilder::new_dynamic().build());
            physics.colliders.insert(ColliderBuilder::cuboid(hs.x,
                                                             hs.y).build(),
                                     body, &mut physics.bodies);
            world.insert(entity,
                         (Global2::identity(), body,
                          Sprite{pos:
                                     Rect{left: -hs.x,
                                          right: hs.x,
                                          top: -hs.y,
                                          bottom: hs.y,},
                                 uv:
                                     Rect{left: 0.0,
                                          right: 1.0,
                                          top: 0.0,
                                          bottom: 1.0,},
                                 layer: 1,},
                          Material{albedo_coverage:
                                       Some(Texture{image:
                                                        sprite_sheet.image.clone(),
                                                    sampler,}),
                                   albedo_factor:
                                       [OrderedFloat(color[0]),
                                        OrderedFloat(color[1]),
                                        OrderedFloat(color[2])],
                                                                   ..Default::default()},
                          SpriteAnimState::new(sprite_sheet),
                          ContactQueue2::new()))?;
            Ok(())
        }
    }
    struct TankState {
        speed: f32,
        moment: f32,
        fire: bool,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for TankState {
        #[inline]
        fn clone(&self) -> TankState {
            {
                let _: ::core::clone::AssertParamIsClone<f32>;
                let _: ::core::clone::AssertParamIsClone<f32>;
                let _: ::core::clone::AssertParamIsClone<bool>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TankState { }
    pub struct ControlledTank {
        state: TankState,
        newstate: TankState,
    }
    pub struct TankController {
        forward: VirtualKeyCode,
        backward: VirtualKeyCode,
        left: VirtualKeyCode,
        right: VirtualKeyCode,
        fire: VirtualKeyCode,
        forward_pressed: bool,
        backward_pressed: bool,
        left_pressed: bool,
        right_pressed: bool,
    }
    impl TankController {
        pub fn main() -> Self {
            TankController{forward: VirtualKeyCode::W,
                           backward: VirtualKeyCode::S,
                           left: VirtualKeyCode::A,
                           right: VirtualKeyCode::D,
                           fire: VirtualKeyCode::Space,
                           forward_pressed: false,
                           backward_pressed: false,
                           left_pressed: false,
                           right_pressed: false,}
        }
        pub fn alt() -> Self {
            TankController{forward: VirtualKeyCode::Up,
                           backward: VirtualKeyCode::Down,
                           left: VirtualKeyCode::Left,
                           right: VirtualKeyCode::Right,
                           fire: VirtualKeyCode::Insert,
                           forward_pressed: false,
                           backward_pressed: false,
                           left_pressed: false,
                           right_pressed: false,}
        }
    }
    impl InputController for TankController {
        type Controlled = ControlledTank;
        fn controlled(&self) -> ControlledTank {
            ControlledTank{state:
                               TankState{speed: 0.0,
                                         moment: 0.0,
                                         fire: false,},
                           newstate:
                               TankState{speed: 0.0,
                                         moment: 0.0,
                                         fire: false,},}
        }
        fn control(&mut self, event: DeviceEvent, tank: &mut ControlledTank)
         -> ControlResult {
            match event {
                DeviceEvent::Key(KeyboardInput {
                                 state, virtual_keycode: Some(key), .. }) => {
                    let pressed =
                        match state {
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        };
                    if key == self.forward {
                        self.forward_pressed = pressed;
                    } else if key == self.backward {
                        self.backward_pressed = pressed;
                    } else if key == self.left {
                        self.left_pressed = pressed;
                    } else if key == self.right {
                        self.right_pressed = pressed;
                    } else if key == self.fire {
                        tank.newstate.fire = state == ElementState::Pressed;
                    } else { return ControlResult::Ignored; }
                    tank.newstate.speed =
                        3.0 *
                            (self.forward_pressed as u8 as f32 -
                                 self.backward_pressed as u8 as f32);
                    tank.newstate.moment =
                        3.0 *
                            (self.right_pressed as u8 as f32 -
                                 self.left_pressed as u8 as f32);
                    ControlResult::Consumed
                }
                _ => ControlResult::Ignored,
            }
        }
    }
    pub struct TankSystem;
    impl System for TankSystem {
        fn name(&self) -> &str { "TankSystem" }
        fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
            let physics = cx.res.with(PhysicsData2::new);
            let mut bullets = BVec::new_in(cx.bump);
            let mut despawn = BVec::new_in(cx.bump);
            'e:
                for (entity, (body, global, tank, state, queue)) in
                    cx.world.query::<(&RigidBodyHandle, &Global2,
                                      &mut ControlledTank,
                                      Option<&mut SpriteAnimState>,
                                      &mut ContactQueue2)>().with::<Tank>().iter()
                    {
                    for collider in queue.drain_contacts_started() {
                        let bits =
                            physics.colliders.get(collider).unwrap().user_data
                                as u64;
                        let bullet =
                            cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();
                        if bullet {
                            despawn.push(entity);
                            physics.bodies.remove(*body,
                                                  &mut physics.colliders,
                                                  &mut physics.joints);
                            continue 'e ;
                        }
                    }
                    if let Some(state) = state {
                        if tank.newstate.speed > 0.1 &&
                               tank.state.speed <= 0.1 {
                            state.set_anim(Anim::Loop{animation: 1,});
                        }
                        if tank.newstate.speed <= 0.1 &&
                               tank.state.speed > 0.1 {
                            state.set_anim(Anim::Loop{animation: 0,});
                        }
                    }
                    if let Some(body) = physics.bodies.get_mut(*body) {
                        let vel = na::Vector2::new(0.0, -tank.newstate.speed);
                        let vel = global.iso.rotation.transform_vector(&vel);
                        body.set_linvel(vel, true);
                        body.set_angvel(tank.newstate.moment, true);
                    }
                    if tank.newstate.fire {
                        let pos =
                            global.iso.transform_point(&na::Point2::new(0.0,
                                                                        -0.6));
                        let dir =
                            global.iso.transform_vector(&na::Vector2::new(0.0,
                                                                          -10.0));
                        bullets.push((pos, dir));
                        tank.newstate.fire = false;
                    }
                    tank.state = tank.newstate;
                }
            for entity in despawn {
                if let Ok(iso) =
                       cx.world.get::<Global2>(entity).map(|g| g.iso) {
                    cx.world.spawn((Global2::new(iso),
                                    Sprite{pos:
                                               Rect{left: -0.5,
                                                    right: 0.5,
                                                    top: -0.5,
                                                    bottom: 0.5,},
                                           uv:
                                               Rect{left: 0.0,
                                                    right: 1.0,
                                                    top: 0.0,
                                                    bottom: 1.0,},
                                           layer: 0,},
                                    Material{albedo_factor:
                                                 [OrderedFloat(0.7),
                                                  OrderedFloat(0.1),
                                                  OrderedFloat(0.1)],
                                                                        ..Default::default()}));
                }
                let _ = cx.world.despawn(entity);
            }
            if !bullets.is_empty() {
                let collider = cx.res.with(BulletCollider::new).0.clone();
                let physics = cx.res.with(PhysicsData2::new);
                for (pos, dir) in bullets {
                    let body =
                        physics.bodies.insert(RigidBodyBuilder::new_dynamic().build());
                    physics.colliders.insert(collider.clone(), body,
                                             &mut physics.bodies);
                    physics.bodies.get_mut(body).unwrap().set_linvel(dir,
                                                                     true);
                    cx.world.spawn((Global2::new(na::Translation2::new(pos.x,
                                                                       pos.y).into()),
                                    Bullet, body,
                                    Sprite{pos:
                                               Rect{left: -0.05,
                                                    right: 0.05,
                                                    top: -0.05,
                                                    bottom: 0.05,},
                                           uv:
                                               Rect{left: 0.0,
                                                    right: 1.0,
                                                    top: 0.0,
                                                    bottom: 1.0,},
                                           layer: 0,},
                                    Material{albedo_factor:
                                                 [OrderedFloat(1.0),
                                                  OrderedFloat(0.8),
                                                  OrderedFloat(0.2)],
                                                                        ..Default::default()},
                                    ContactQueue2::new()));
                }
            }
            Ok(())
        }
    }
    pub struct SpriteAnimationSystem;
    impl System for SpriteAnimationSystem {
        fn name(&self) -> &str { "SpriteAnimationSystem" }
        fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
            for (_, (state, sprite)) in
                cx.world.query_mut::<(&mut SpriteAnimState, &mut Sprite)>() {
                state.advance(cx.clock.delta);
                sprite.uv = state.get_frame().rect;
            }
            Ok(())
        }
    }
    struct SpriteAnimState {
        current_animation: usize,
        current_frame: usize,
        current_frame_time_us: u64,
        anim: Anim,
        frames: Vec<Frame>,
        animations: Vec<Animation>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for SpriteAnimState {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                SpriteAnimState {
                current_animation: ref __self_0_0,
                current_frame: ref __self_0_1,
                current_frame_time_us: ref __self_0_2,
                anim: ref __self_0_3,
                frames: ref __self_0_4,
                animations: ref __self_0_5 } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f,
                                                                  "SpriteAnimState");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "current_animation",
                                                        &&(*__self_0_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "current_frame",
                                                        &&(*__self_0_1));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "current_frame_time_us",
                                                        &&(*__self_0_2));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "anim",
                                                        &&(*__self_0_3));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "frames",
                                                        &&(*__self_0_4));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "animations",
                                                        &&(*__self_0_5));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl SpriteAnimState {
        fn new(sheet: &SpriteSheet) -> Self {
            SpriteAnimState{current_animation: 0,
                            current_frame: 0,
                            current_frame_time_us: 0,
                            anim: Anim::Loop{animation: 0,},
                            frames: sheet.frames.clone(),
                            animations: sheet.animations.clone(),}
        }
        fn set_anim(&mut self, anim: Anim) {
            match anim {
                Anim::Loop { animation } => {
                    self.anim = anim;
                    self.current_animation = animation;
                    self.current_frame = 0;
                    self.current_frame_time_us = 0;
                }
                Anim::RunAndLoop { animation, .. } => {
                    self.anim = anim;
                    self.current_animation = animation;
                    self.current_frame = 0;
                    self.current_frame_time_us = 0;
                }
            }
        }
        fn get_frame(&self) -> &Frame {
            let anim = &self.animations[self.current_animation];
            &self.frames[anim.from..=anim.to][self.current_frame]
        }
        fn advance(&mut self, delta: Duration) {
            let mut delta = delta.as_micros() as u64;
            loop  {
                let anim = &self.animations[self.current_animation];
                let frames = &self.frames[anim.from..=anim.to];
                if self.current_frame_time_us + delta <
                       frames[self.current_frame].duration_us {
                    self.current_frame_time_us += delta;
                    return;
                }
                delta -=
                    frames[self.current_frame].duration_us -
                        self.current_frame_time_us;
                self.current_frame += 1;
                self.current_frame_time_us = 0;
                if frames.len() == self.current_frame {
                    self.current_frame = 0;
                    match self.anim {
                        Anim::Loop { .. } => { }
                        Anim::RunAndLoop { and_loop, .. } => {
                            self.anim = Anim::Loop{animation: and_loop,};
                            self.current_animation = and_loop;
                        }
                    }
                }
            }
        }
    }
    pub enum Anim {

        /// Cycle through animations
        Loop {
            animation: usize,
        },
        RunAndLoop {
            animation: usize,
            and_loop: usize,
        },
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Anim {
        #[inline]
        fn clone(&self) -> Anim {
            {
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Anim { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Anim {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&Anim::Loop { animation: ref __self_0 },) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Loop");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "animation",
                                                        &&(*__self_0));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
                (&Anim::RunAndLoop {
                 animation: ref __self_0, and_loop: ref __self_1 },) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f,
                                                                  "RunAndLoop");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "animation",
                                                        &&(*__self_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder,
                                                        "and_loop",
                                                        &&(*__self_1));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    pub struct BulletSystem;
    impl System for BulletSystem {
        fn name(&self) -> &str { "BulletSystem" }
        fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
            let physics = cx.res.with(PhysicsData2::new);
            let mut despawn = BVec::new_in(cx.bump);
            for (e, (queue, body)) in
                cx.world.query_mut::<(&mut ContactQueue2,
                                      &RigidBodyHandle)>().with::<Bullet>() {
                if queue.drain_contacts_started().count() > 0 {
                    despawn.push(e);
                    physics.bodies.remove(*body, &mut physics.colliders,
                                          &mut physics.joints);
                }
                queue.drain_contacts_stopped();
            }
            for e in despawn {
                if let Ok(iso) = cx.world.get::<Global2>(e).map(|g| g.iso) {
                    cx.world.spawn((Global2::new(iso),
                                    Sprite{pos:
                                               Rect{left: -0.2,
                                                    right: 0.2,
                                                    top: -0.2,
                                                    bottom: 0.2,},
                                           uv:
                                               Rect{left: 0.0,
                                                    right: 1.0,
                                                    top: 0.0,
                                                    bottom: 1.0,},
                                           layer: 0,},
                                    Material{albedo_factor:
                                                 [OrderedFloat(1.0),
                                                  OrderedFloat(0.3),
                                                  OrderedFloat(0.1)],
                                                                        ..Default::default()}));
                }
                let _ = cx.world.despawn(e);
            }
            Ok(())
        }
    }
}
fn main() {
    game2(|mut game|
              async move
                  {
                      game.loader.load_prefab::<TileMap>("".parse().unwrap(),
                                                         &mut game.world);
                      let tank =
                          game.loader.load_prefab(tank::Tank::new("tank-1.json".into(),
                                                                  na::Vector2::new(0.9,
                                                                                   0.9),
                                                                  [0.2, 0.9,
                                                                   0.2]),
                                                  &mut game.world);
                      let tank2 =
                          game.loader.load_prefab(tank::Tank::new("tank-1.json".into(),
                                                                  na::Vector2::new(0.9,
                                                                                   0.9),
                                                                  [0.9, 0.2,
                                                                   0.2]),
                                                  &mut game.world);
                      game.scheduler.add_fixed_system(Physics2::new(),
                                                      Duration::from_nanos(16_666_666));
                      game.scheduler.add_system(tank::TankSystem);
                      game.scheduler.add_system(tank::SpriteAnimationSystem);
                      game.scheduler.add_system(tank::BulletSystem);
                      let camera = game.viewport.camera();
                      game.world.get_mut::<Camera2>(camera).unwrap().set_scaley(0.2);
                      game.scheduler.add_system(move |cx: SystemContext<'_>|
                                                    {
                                                        if let Ok(global) =
                                                               cx.world.get::<Global2>(tank)
                                                           {
                                                            let target =
                                                                global.iso.translation.vector;
                                                            if let Ok(mut global)
                                                                   =
                                                                   cx.world.get_mut::<Global2>(camera)
                                                               {
                                                                global.iso.translation.vector
                                                                    =
                                                                    global.iso.translation.vector.lerp(&target,
                                                                                                       (cx.clock.delta.as_secs_f32()
                                                                                                            *
                                                                                                            5.0).clamp(0.0,
                                                                                                                       1.0));
                                                            }
                                                        }
                                                    });
                      game.control.assume_control(tank,
                                                  tank::TankController::main(),
                                                  &mut game.world)?;
                      game.control.assume_control(tank2,
                                                  tank::TankController::alt(),
                                                  &mut game.world)?;
                      Ok(game)
                  })
}
