use proc_macro::TokenStream;
use syn::spanned::Spanned;

struct Asset {
    ty: Option<syn::Path>,
}

struct TypeAttributes {
    func: Option<syn::Path>,
}

struct FieldAttributes {
    asset: Option<Asset>,
}

struct Parsed {
    input: syn::DeriveInput,
    attrs: TypeAttributes,
    fields: Vec<FieldAttributes>,
}

fn parse_field_attributes(attrs: &[syn::Attribute]) -> syn::Result<FieldAttributes> {
    let mut asset = None::<Asset>;

    for attr in attrs {
        if attr.path.is_ident("unfold") {
            attr.parse_args_with(|stream: syn::parse::ParseStream| {
                let arg = stream.parse::<syn::Ident>()?;
                match arg {
                    _ if arg == "asset" => {
                        if asset.is_some() {
                            Err(syn::Error::new_spanned(
                                arg,
                                "Unexpected duplicate 'asset' argument",
                            ))
                        } else if stream.is_empty() {
                            asset = Some(Asset { ty: None });
                            Ok(())
                        } else {
                            stream.parse::<syn::Token![:]>()?;
                            let ty = stream.parse::<syn::Path>()?;

                            if stream.is_empty() {
                                asset = Some(Asset { ty: Some(ty) });
                                Ok(())
                            } else {
                                Err(syn::Error::new(stream.span(), "Expected end of arguments"))
                            }
                        }
                    }
                    _ => Err(syn::Error::new_spanned(arg, "Unrecognized argument")),
                }
            })?;
        }
    }

    Ok(FieldAttributes { asset })
}

fn parse_type_attributes(attrs: &[syn::Attribute]) -> syn::Result<TypeAttributes> {
    let mut func = None::<syn::Path>;

    for attr in attrs {
        if attr.path.is_ident("unfold") {
            attr.parse_args_with(|stream: syn::parse::ParseStream| {
                if stream.peek(syn::Token![fn]) {
                    if func.is_some() {
                        return Err(syn::Error::new(
                            stream.span(),
                            "Unexpected duplicate 'fn' argument",
                        ));
                    }

                    let _fn = stream.parse::<syn::Token![fn]>().unwrap();
                    func = Some(stream.parse::<syn::Path>()?);

                    return Ok(());
                }

                Err(syn::Error::new(stream.span(), "Unrecognized argument"))
            })?
        }
    }

    Ok(TypeAttributes { func })
}

pub fn derive_unfold(item: TokenStream) -> syn::Result<TokenStream> {
    let parsed = parse(item)?;

    let ident = &parsed.input.ident;

    let none = parsed.attrs.func.is_none() && parsed.fields.iter().all(|f| f.asset.is_none());

    if none {
        // With no attributes, nothing should be done for the unfold type.
        // Assign dummy system as unfold system.
        return Ok(quote::quote! {
            impl ::arcana::unfold::Unfold for #ident {
                type UnfoldSystem = ::arcana::unfold::DummyUnfoldSystem;
            }
        }
        .into());
    }

    let data = match parsed.input.data {
        syn::Data::Struct(s) => s,
        _ => unreachable!(),
    };

    let system_ident = quote::format_ident!("{ident}UnfoldSystem");
    let system_name = syn::LitStr::new(&format!("{ident} unfold system"), ident.span());
    let system_struct = quote::quote_spanned!(ident.span() => #[derive(Clone, Copy, Debug, Default)] pub struct #system_ident;);
    let unfolded_ident = quote::format_ident!("{ident}UnfoldSpawned");

    let stream = match &parsed.attrs.func {
        None => {
            let mut unfolded_fields = Vec::new();
            let mut unfolded_fields_init = Vec::new();

            let mut cleanup_statements = Vec::new();
            let mut updates = Vec::new();

            let asset_fields = parsed.fields.iter().enumerate().filter_map(|(i, f)| {
                let asset = f.asset.as_ref()?;
                Some((i, asset))
            });

            for (spawned_idx, (value_idx, asset)) in asset_fields.enumerate() {
                let field = data.fields.iter().nth(value_idx).unwrap();
                let ty = &field.ty;

                let asset_ty = match &asset.ty {
                    None => {
                        quote::quote_spanned!(ty.span() => <#ty as TypedAssetIdExt>::Asset)
                    }
                    Some(ty) => {
                        quote::quote_spanned!(ty.span() => #ty)
                    }
                };

                cleanup_statements.push(
                    quote::quote_spanned!(ty.span() => let _ = cx.world.remove::<#asset_ty>(&e);),
                );

                let unfolded_field_ident = match &field.ident {
                    None => syn::Member::Unnamed(syn::Index {
                        span: field.span(),
                        index: spawned_idx as u32,
                    }),
                    Some(field_ident) => syn::Member::Named(field_ident.clone()),
                };

                let value_field_ident = match &field.ident {
                    None => syn::Member::Unnamed(syn::Index {
                        span: field.span(),
                        index: value_idx as u32,
                    }),
                    Some(field_ident) => syn::Member::Named(field_ident.clone()),
                };

                if field.ident.is_some() {
                    unfolded_fields
                        .push(quote::quote_spanned!(field.span() => #unfolded_field_ident: ::core::option::Option<::arcana::assets::AssetId>));
                    unfolded_fields_init
                        .push(quote::quote_spanned!(field.span() => #unfolded_field_ident: ::core::option::Option::None));
                } else {
                    unfolded_fields
                        .push(quote::quote_spanned!(field.span() => ::core::option::Option<::arcana::assets::AssetId>));
                    unfolded_fields_init
                        .push(quote::quote_spanned!(field.span() => ::core::option::Option::None));
                }

                updates.push(quote::quote_spanned!(field.span() => {
                    let id = *Borrow::<AssetId>::borrow(&value.#value_field_ident);
                    match unfolded.#unfolded_field_ident {
                        Some(old_id) if old_id == id => {}
                        _ => match cx.assets.build::<#asset_ty, _>(id, cx.graphics) {
                            None => {},
                            Some(Ok(asset)) => {
                                unfolded.#unfolded_field_ident = Some(id);
                                entity_builder.add(<#asset_ty as Clone>::clone(asset));
                            }
                            Some(Err(err)) => {
                                tracing::error!("Failed to load asset '{}({:})'. {:#}", type_name::<#asset_ty>(), id, err);
                                unfolded.#unfolded_field_ident = Some(id);
                            }
                        },
                    }
                }));
            }

            let unfolded_struct = quote::quote_spanned!(ident.span() => struct #unfolded_ident {
                #(#unfolded_fields,)*
            });

            quote::quote! {
                impl ::arcana::unfold::Unfold for #ident {
                    type UnfoldSystem = #system_ident;
                }

                #system_struct

                #unfolded_struct

                impl ::arcana::system::System for #system_ident {
                    fn name(&self) -> &str {
                        #system_name
                    }

                    fn run(&mut self, cx: ::arcana::system::SystemContext<'_>) {
                        use core::{any::type_name, borrow::Borrow, clone::Clone, option::Option::{self, None, Some}};
                        use std::vec::Vec;
                        use arcana::{assets::{AssetId, TypedAssetIdExt}, edict::entity::{EntityId, EntityBuilder}};

                        let cleanup_query = cx.world.query_mut::<()>().with::<#unfolded_ident>().without::<#ident>();

                        let mut cleanup = Vec::new_in(&*cx.scope);
                        cleanup.extend(cleanup_query.into_iter().map(|(e, ())| e));

                        for e in cleanup {
                            let _ = cx.world.remove::<#unfolded_ident>(&e);
                            #( #cleanup_statements )*
                        }

                        let query = cx.world.query_mut::<(&#ident, Option<&mut #unfolded_ident>)>();

                        let mut inserts = Vec::new_in(&*cx.scope);

                        for (e, (value, unfolded)) in query {
                            let mut unfolded_insert = None;

                            let unfolded: &mut #unfolded_ident = match unfolded {
                                None => {
                                    unfolded_insert.get_or_insert(#unfolded_ident {
                                        #(#unfolded_fields_init,)*
                                    })
                                }
                                Some(unfolded) => unfolded,
                            };

                            let mut entity_builder = EntityBuilder::new();

                            #(#updates;)*

                            if let Some(unfolded_insert) = unfolded_insert {
                                entity_builder.add(unfolded_insert);
                            }

                            if !entity_builder.is_empty() {
                                inserts.push((e, entity_builder));
                            }
                        }

                        for (e, mut entity_builder) in inserts {
                            cx.world.try_insert(&e, entity_builder).unwrap();
                        }
                    }
                }
            }
        }
        Some(unfold) => {
            let mut unfolded_fields = Vec::new();
            let mut unfolded_fields_init = Vec::new();

            let mut updates = Vec::new();

            let mut unfold_fn_arg_types = Vec::new();
            let mut unfold_fn_args = Vec::new();

            for (idx, f) in parsed.fields.iter().enumerate() {
                let field = data.fields.iter().nth(idx).unwrap();
                let ty = &field.ty;

                let field_ident = match &field.ident {
                    None => syn::Member::Unnamed(syn::Index {
                        span: field.span(),
                        index: idx as u32,
                    }),
                    Some(field_ident) => syn::Member::Named(field_ident.clone()),
                };

                match &f.asset {
                    None => {
                        if field.ident.is_some() {
                            unfolded_fields
                                .push(quote::quote_spanned!(field.span() => #field_ident: #ty));
                            unfolded_fields_init.push(
                                quote::quote_spanned!(field_ident.span() => #field_ident: Clone::clone(&value.#field_ident)),
                            );
                        } else {
                            unfolded_fields.push(quote::quote_spanned!(field.span() => #ty));
                            unfolded_fields_init.push(
                                quote::quote_spanned!(field_ident.span() => Clone::clone(&value.#field_ident))
                            );
                        }

                        unfold_fn_arg_types.push(quote::quote_spanned!(field.span() => &#ty));
                        unfold_fn_args
                            .push(quote::quote_spanned!(field.span() => &unfolded.#field_ident));

                        updates.push(quote::quote_spanned!(field.span() => {
                            if value.#field_ident != unfolded.#field_ident {
                                updated = true;
                            }
                        }));
                    }
                    Some(asset) => {
                        let asset_ty = match &asset.ty {
                            None => {
                                quote::quote_spanned!(ty.span() => <#ty as ::arcana::assets::TypedAssetIdExt>::Asset)
                            }
                            Some(ty) => {
                                quote::quote_spanned!(ty.span() => #ty)
                            }
                        };

                        if field.ident.is_some() {
                            unfolded_fields.push(quote::quote_spanned!(field.span() => #field_ident: ::core::option::Option< ::core::result::Result< ::arcana::assets::WithId<#asset_ty>, ::arcana::assets::AssetId > >));
                            unfolded_fields_init.push(
                                quote::quote_spanned!(field_ident.span() => #field_ident: None),
                            );
                        } else {
                            unfolded_fields.push(
                                quote::quote_spanned!(field.span() => ::core::option::Option<::arcana::assets::WithId<#asset_ty>>),
                            );
                            unfolded_fields_init
                                .push(quote::quote_spanned!(field_ident.span() => None));
                        }

                        unfold_fn_arg_types
                            .push(quote::quote_spanned!(field.span() => &::arcana::assets::WithId<#asset_ty>));
                        unfold_fn_args
                            .push(quote::quote_spanned!(field.span() => unfolded.#field_ident.as_ref().unwrap().as_ref().unwrap()));

                        updates.push(quote::quote_spanned!(field.span() => {
                            let id = *Borrow::<AssetId>::borrow(&value.#field_ident);
                            match &unfolded.#field_ident {
                                Some(Ok(old_id)) if WithId::id(old_id) == id => {}
                                Some(Err(old_id)) if *old_id == id => {}
                                _ => match cx.assets.build::<#asset_ty, _>(id, cx.graphics) {
                                    None => {
                                        ready = false;
                                    },
                                    Some(Ok(asset)) => {
                                        updated = true;
                                        unfolded.#field_ident = Some(Ok(WithId::new(Clone::clone(asset), id)));
                                    }
                                    Some(Err(err)) => {
                                        ready = false;

                                        ::arcana::tracing::error!("Failed to load asset '{}({:})'. {:#}", type_name::<#asset_ty>(), id, err);
                                        unfolded.#field_ident = Some(Err(id));
                                    }
                                },
                            }
                        }));
                    }
                }
            }

            let unfolded_struct = quote::quote_spanned!(ident.span() => struct #unfolded_ident {
                #(#unfolded_fields,)*
            });

            quote::quote! {
                impl ::arcana::unfold::Unfold for #ident {
                    type UnfoldSystem = #system_ident;
                }

                #system_struct

                #unfolded_struct

                impl ::arcana::system::System for #system_ident {
                    fn name(&self) -> &str {
                        #system_name
                    }

                    fn run(&mut self, cx: ::arcana::system::SystemContext<'_>) {
                        use core::{any::type_name, borrow::Borrow, clone::Clone, iter::Iterator, option::Option::{self, Some, None}};
                        use std::vec::Vec;
                        use ::arcana::{assets::WithId, edict::{bundle::Bundle, entity::EntityId, world::World}, unfold::UnfoldResult, resources::Res};

                        let cleanup_query = cx.world.query_mut::<&#unfolded_ident>().without::<#ident>();

                        let mut cleanup = Vec::new_in(&*cx.scope);
                        cleanup.extend(cleanup_query.into_iter().map(|(e, _)| e));

                        for e in cleanup {
                            let _ = cx.world.remove::<#unfolded_ident>(&e);

                            fn cleanup<T: Bundle, I>(world: &mut World, entity: &EntityId, _: fn( #(#unfold_fn_arg_types,)* &mut Res ) -> UnfoldResult<T, I>) {
                                let _ = world.remove_bundle::<T>(entity);
                            }

                            cleanup(cx.world, &e, #unfold);
                        }

                        let query = cx.world.query_mut::<(&#ident, Option<&mut #unfolded_ident>)>();

                        let mut unfolded_inserts = Vec::new_in(&*cx.scope);
                        let mut inserts = Vec::new_in(&*cx.scope);
                        let mut spawns = Vec::new_in(&*cx.scope);

                        for (e, (value, unfolded)) in query {
                            let mut unfolded_insert = None;
                            let mut ready = true;
                            let mut updated = false;

                            let unfolded: &mut #unfolded_ident = match unfolded {
                                None => {
                                    updated = true;
                                    unfolded_insert.get_or_insert(#unfolded_ident {
                                        #(#unfolded_fields_init,)*
                                    })
                                }
                                Some(unfolded) => unfolded,
                            };

                            #(#updates;)*


                            if updated && ready {
                                let UnfoldResult { insert, spawn } = (#unfold)( #(#unfold_fn_args,)* cx.res );
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
                            todo!()
                        }
                    }
                }
            }
        }
    };

    Ok(stream.into())
}

fn parse(item: TokenStream) -> syn::Result<Parsed> {
    let input = syn::parse::<syn::DeriveInput>(item)?;

    match &input.data {
        syn::Data::Enum(data) => Err(syn::Error::new_spanned(
            data.enum_token,
            "Enumerations are unsupported by `Unfold` derive macro",
        )),
        syn::Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "Unions are unsupported by `Unfold` derive macro",
        )),
        syn::Data::Struct(data) => {
            let attrs = parse_type_attributes(&input.attrs)?;

            let fields = data
                .fields
                .iter()
                .map(|f| parse_field_attributes(&f.attrs))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Parsed {
                input,
                attrs,
                fields,
            })
        }
    }
}
