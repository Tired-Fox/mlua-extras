use std::sync::atomic::{AtomicUsize, Ordering};

use proc_macro2::{Span, TokenStream};
use syn::{parse_quote, Attribute, FnArg, Ident, ImplItem, ItemImpl, Pat, Result, Signature, Type};

use crate::userdata::{
    attr::{parse_attrs, validate_method_attr},
    collect_docs, with_cfg,
};

static BORROW_WRAPPERS: &[(&str, &str)] = &[
    ("str", "::mlua_extras::mlua::string::BorrowedStr"),
    ("[u8]", "::mlua_extras::mlua::string::BorrowedBytes"),
];

enum Instance {
    Ref(RefKind),
    Owned,
    None,
}

enum RefKind {
    Ref,
    Mut,
    OptionRef,
    OptionMut,
}

struct ArgInfo {
    ident: Ident,
    userdata_ref: Option<RefKind>,
    callback_type: Type,
    variadic: bool,
    docs: Option<String>,
}

enum Lua {
    Ref,
    Owned,
}

struct MethodInfo {
    instance: Instance,
    lua: Option<Lua>,
    args: Vec<ArgInfo>,
}

/// Extract inner type from a reference type
fn ref_inner_type(ty: &Type) -> Type {
    match ty {
        Type::Reference(ref_ty) => (*ref_ty.elem).clone(),
        _ => ty.clone(),
    }
}

/// Validate if type is mlua::Lua
fn is_lua_type(ty: &Type) -> bool {
    let Type::Path(p) = ty else { return false };
    match p.path.segments.len() {
        1 => p.path.segments[0].ident == "Lua",
        2 => p.path.segments[0].ident == "mlua" && p.path.segments[1].ident == "Lua",
        3 => {
            p.path.segments[0].ident == "mlua_extras"
                && p.path.segments[1].ident == "mlua"
                && p.path.segments[2].ident == "Lua"
        }
        _ => false,
    }
}

/// Classify the moethod's `Lua` context parameter, if present.
fn lua_arg_kind(ty: &Type) -> Option<Lua> {
    match ty {
        Type::Reference(r) if r.mutability.is_none() && is_lua_type(&r.elem) => Some(Lua::Ref),
        ty if is_lua_type(ty) => Some(Lua::Owned),
        _ => None,
    }
}

fn classify_ref(ty: &Type) -> Option<Type> {
    let Type::Reference(ref_ty) = ty else {
        return None;
    };

    if ref_ty.mutability.is_none() {
        let lookup_name: Option<String> = match &*ref_ty.elem {
            Type::Path(path) => path.path.segments.last().map(|seg| seg.ident.to_string()),
            Type::Slice(slice) => {
                if let Type::Path(path) = &*slice.elem {
                    path.path
                        .segments
                        .last()
                        .map(|seg| format!("[{}]", seg.ident))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(ref name) = lookup_name {
            for &(inner, wrapper) in BORROW_WRAPPERS {
                if name == inner {
                    let wrapper = syn::parse_str(wrapper).expect("invalid wrapper type");
                    return Some(wrapper);
                }
            }
        }
    }

    // Mutable references to slices are not supported
    if matches!(&*ref_ty.elem, Type::Slice(_)) && ref_ty.mutability.is_some() {
        return None;
    }

    let inner = ref_inner_type(ty);
    if ref_ty.mutability.is_none() {
        Some(parse_quote! { ::mlua_extras::mlua::userdata::UserDataRef<#inner> })
    } else {
        Some(parse_quote! { ::mlua_extras::mlua::userdata::UserDataRefMut<#inner> })
    }
}

fn try_unwrap_option(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }

    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    let syn::GenericArgument::Type(inner) = &args.args[0] else {
        return None;
    };
    Some(inner)
}

fn classify_variadic(ty: &Type) -> bool {
    let Type::Path(p) = ty else { return false };
    match p.path.segments.len() {
        1 => p.path.segments[0].ident == "Variadic",
        2 => p.path.segments[0].ident == "mlua" && p.path.segments[1].ident == "Variadic",
        3 => {
            p.path.segments[0].ident == "mlua_extras"
                && p.path.segments[1].ident == "mlua"
                && p.path.segments[2].ident == "Variadic"
        }
        _ => false,
    }
}

fn parse_signature(sig: &mut Signature) -> Result<MethodInfo> {
    let mut instance = Instance::None;
    let mut lua = None;
    let mut args = Vec::new();
    let mut check_first_typed = true;

    for param in &mut sig.inputs {
        match param {
            FnArg::Receiver(recv) if recv.reference.is_some() && recv.mutability.is_some() => {
                instance = Instance::Ref(RefKind::Mut);
            }
            FnArg::Receiver(recv) if recv.reference.is_some() => {
                instance = Instance::Ref(RefKind::Ref);
            }
            FnArg::Receiver(_) => {
                instance = Instance::Owned;
            }
            FnArg::Typed(typed) => {
                if check_first_typed {
                    check_first_typed = false;
                    if let Some(kind) = lua_arg_kind(&typed.ty) {
                        lua = Some(kind);
                        continue;
                    }
                }

                let ident = match &*typed.pat {
                    Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                    Pat::Wild(_) => {
                        // For wildcards we generate a unique identifier to avoid collisions with other params
                        Ident::new(&format!("__mlua_arg_{}", args.len()), Span::mixed_site())
                    },
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &typed.pat,
                            "`#[mlua_extras::typeduserdata_impl]` requires a named parameter or `_`; destructuring patterns are not supported",
                        ))
                    }
                };

                let arg_type = &*typed.ty;
                let mut option_inner = None;
                let ref_kind = match arg_type {
                    Type::Reference(r) if r.mutability.is_some() => Some(RefKind::Mut),
                    Type::Reference(_) => Some(RefKind::Ref),
                    _ => {
                        option_inner = try_unwrap_option(arg_type);
                        option_inner.and_then(|inner| match inner {
                            Type::Reference(r) if r.mutability.is_some() => {
                                Some(RefKind::OptionMut)
                            }
                            Type::Reference(_) => Some(RefKind::OptionRef),
                            _ => None,
                        })
                    }
                };
                let callback_type = match &ref_kind {
                    Some(RefKind::OptionRef | RefKind::OptionMut) => {
                        match classify_ref(option_inner.unwrap()) {
                            Some(ty) => parse_quote! { Option<#ty> },
                            None => {
                                return Err(syn::Error::new_spanned(
                                    arg_type,
                                    "this reference type is not supported as a callback parameter",
                                ));
                            }
                        }
                    }
                    Some(_) => match classify_ref(arg_type) {
                        Some(ty) => ty,
                        None => {
                            return Err(syn::Error::new_spanned(
                                arg_type,
                                "this reference type is not supported as a callback parameter",
                            ));
                        }
                    },
                    None => arg_type.clone(),
                };

                let is_variadic = classify_variadic(arg_type);

                let docs = collect_docs(&typed.attrs);
                typed.attrs.retain(|a| !a.path().is_ident("doc"));

                args.push(ArgInfo {
                    ident,
                    variadic: is_variadic,
                    userdata_ref: ref_kind,
                    callback_type,
                    docs,
                });
            }
        }
    }

    Ok(MethodInfo {
        instance,
        lua,
        args,
    })
}

fn strip_lua_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("lua"))
        .cloned()
        .collect()
}

pub fn derive(input: &mut ItemImpl) -> TokenStream {
    if !input.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &input.generics,
            "`#[mlua::userdata_impl]` does not support generic impl blocks",
        )
        .to_compile_error()
        .into();
    }

    if let Some(where_clause) = &input.generics.where_clause {
        return syn::Error::new_spanned(
            where_clause,
            "`#[mlua_extras::typeduserdata_impl]` does not support `where` clauses on the impl block"
        )
        .to_compile_error()
        .into();
    }

    let type_path = match &*input.self_ty {
        Type::Path(type_path) => &type_path.path,
        _ => {
            return syn::Error::new_spanned(
                &input.self_ty,
                "`#[typeduserdata_impl]` requires a simple path type",
            )
            .to_compile_error()
            .into();
        }
    };
    let type_name = (type_path.segments)
        .last()
        .map(|seg| seg.ident.clone())
        .ok_or_else(|| syn::Error::new_spanned(&input.self_ty, "cannot determine type name"));
    let type_name = match type_name {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique_suffix = COUNTER.fetch_add(1, Ordering::Relaxed);
    let register_fn_name = format_ident!("__mlua_register_{type_name}_{unique_suffix}");
    let register_fn_name_unwrapped =
        format_ident!("__mlua_register_{type_name}_unwrapped_{unique_suffix}");
    let register_fn_name_wrapped =
        format_ident!("__mlua_register_{type_name}_wrapped_{unique_suffix}");
    let registration_type_name = format_ident!("__MluaTypedUserDataRegistration_{type_name}");

    let mut registration_calls = Vec::new();
    for item in &mut input.items {
        match item {
            ImplItem::Const(const_item) => {
                let lua_attr = match parse_attrs(&const_item.attrs, validate_method_attr) {
                    Ok(v) => v,
                    Err(e) => return e.to_compile_error().into(),
                };
                if lua_attr.skip {
                    continue;
                }

                if let Some(docs) = collect_docs(&const_item.attrs) {
                    registration_calls.push(quote!{ ::mlua_extras::typed::TypedDataFields::document(registry, #docs); });
                }

                if lua_attr.getter || lua_attr.setter {
                    return syn::Error::new(
                        lua_attr.span(),
                        "const items do not support `getter` or `setter`",
                    )
                    .to_compile_error()
                    .into();
                }

                let const_name = &const_item.ident;
                let lua_name = lua_attr.name_or_default(const_name);
                let tokens = if lua_attr.meta {
                    quote! {
                        registry.add_meta_field(#lua_name, #type_path::#const_name);
                    }
                } else {
                    quote! {
                        registry.add_field(#lua_name, #type_path::#const_name);
                    }
                };
                registration_calls.push(with_cfg(tokens, &const_item.attrs));
            }
            ImplItem::Fn(method) => {
                let lua_attr = match parse_attrs(&method.attrs, validate_method_attr) {
                    Ok(v) => v,
                    Err(e) => return e.to_compile_error().into(),
                };
                if lua_attr.skip {
                    continue;
                }

                if let Some(docs) = collect_docs(&method.attrs) {
                    registration_calls.push(quote!{ ::mlua_extras::typed::TypedDataMethods::document(registry, #docs); });
                }

                let primary = [lua_attr.getter, lua_attr.setter, lua_attr.field];
                let primary_count = primary.iter().filter(|&&x| x).count();
                if primary_count > 1 {
                    return syn::Error::new(
                        lua_attr.span(),
                        "at most one of `getter`, `setter`, `field` can be specified",
                    )
                    .to_compile_error()
                    .into();
                }

                if lua_attr.meta && primary_count == 1 && !lua_attr.field {
                    return syn::Error::new(
                        lua_attr.span(),
                        "`meta` can only be combined with `field`",
                    )
                    .to_compile_error()
                    .into();
                }

                let info = match parse_signature(&mut method.sig) {
                    Ok(v) => v,
                    Err(e) => return e.to_compile_error().into(),
                };
                let fn_name = &method.sig.ident;
                let is_async = method.sig.asyncness.is_some();

                for arg in &info.args {
                    gen_arg_metadata(&mut registration_calls, arg);
                }

                if !is_async && matches!(info.lua, Some(Lua::Owned)) {
                    return syn::Error::new_spanned(
                        &method.sig,
                        "owned `Lua` parameter is only supported for `async` methods (use `&Lua` instead)"
                    )
                    .to_compile_error()
                    .into();
                }

                if lua_attr.getter {
                    if is_async {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "async field getter is not supported",
                        )
                        .to_compile_error()
                        .into();
                    }
                    if !matches!(info.instance, Instance::Ref(RefKind::Ref)) {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "field getter must take `&self`",
                        )
                        .to_compile_error()
                        .into();
                    }
                    if !info.args.is_empty() {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "field getter must not take additional arguments",
                        )
                        .to_compile_error()
                        .into();
                    }

                    let lua_name = lua_attr.name_or_default(fn_name);
                    let call_args = gen_call_args(&info);
                    let this = Ident::new("this", Span::mixed_site());
                    let lua = Ident::new("lua", Span::mixed_site());

                    let tokens = if lua_attr.infallible {
                        quote! {
                            registry.add_field_method_get(#lua_name, |#lua, #this| {
                                let _ = #lua; // silence unused variable warning
                                Ok(#type_path::#fn_name(#call_args))
                            });
                        }
                    } else {
                        quote! {
                            registry.add_field_method_get(#lua_name, |#lua, #this| {
                                let _ = #lua; // silence unused variable warning
                                #type_path::#fn_name(#call_args)
                            });
                        }
                    };

                    registration_calls.push(with_cfg(tokens, &method.attrs));
                    continue;
                }
                if lua_attr.setter {
                    if is_async {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "async field setter is not supported",
                        )
                        .to_compile_error()
                        .into();
                    }
                    if !matches!(info.instance, Instance::Ref(_)) {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "field setter must take `&[mut] self`",
                        )
                        .to_compile_error()
                        .into();
                    }
                    if info.args.len() != 1 {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "field setter must take exactly one value argument",
                        )
                        .to_compile_error()
                        .into();
                    }

                    let lua_name = lua_attr.name_or_default(fn_name);
                    let call_args = gen_call_args(&info);
                    let this = Ident::new("this", Span::mixed_site());
                    let lua = Ident::new("lua", Span::mixed_site());
                    let val_ident = info.args.first().map(|a| &a.ident);
                    let tokens = if lua_attr.infallible {
                        quote! {
                            registry.add_field_method_set(#lua_name, |#lua, #this, #val_ident| {
                                let _ = #lua; // silence unused variable warning
                                Ok(#type_path::#fn_name(#call_args))
                            });
                        }
                    } else {
                        quote! {
                            registry.add_field_method_set(#lua_name, |#lua, #this, #val_ident| {
                                let _ = #lua; // silence unused variable warning
                                #type_path::#fn_name(#call_args)
                            });
                        }
                    };

                    registration_calls.push(with_cfg(tokens, &method.attrs));
                    continue;
                }
                if lua_attr.field {
                    if is_async {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "async field function is not supported",
                        )
                        .to_compile_error()
                        .into();
                    }
                    if !matches!(info.instance, Instance::None) {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "field function must not take `self`",
                        )
                        .to_compile_error()
                        .into();
                    }
                    if !info.args.is_empty() {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "field function must not take arguments",
                        )
                        .to_compile_error()
                        .into();
                    }

                    let lua_name = lua_attr.name_or_default(fn_name);
                    let tokens = if lua_attr.meta {
                        quote! {
                            registry.add_meta_field(#lua_name, #type_path::#fn_name());
                        }
                    } else {
                        quote! {
                            registry.add_field(#lua_name, #type_path::#fn_name());
                        }
                    };
                    registration_calls.push(with_cfg(tokens, &method.attrs));
                    continue;
                }

                if lua_attr.meta {
                    if matches!(info.instance, Instance::Owned) {
                        return syn::Error::new_spanned(
                            &method.sig,
                            "meta methods cannot take `self`, use `&[mut] self` instead",
                        )
                        .to_compile_error()
                        .into();
                    }

                    if is_async {
                        let meta_name = match lua_attr.meta_name(fn_name) {
                            Ok(name) => name,
                            Err(err) => return err.to_compile_error().into(),
                        };
                        let closure_params = gen_async_closure_params(&info);
                        let call_args = gen_async_call_args(&info);
                        let fn_path = quote! { #type_path::#fn_name };

                        let body = if lua_attr.infallible {
                            quote! { async move { Ok(#fn_path(#call_args).await) } }
                        } else {
                            quote! { async move { #fn_path(#call_args).await } }
                        };

                        let tokens = match info.instance {
                            Instance::None => quote! {
                                registry.add_async_meta_function(#meta_name, #closure_params #body);
                            },
                            Instance::Ref(RefKind::Mut) => quote! {
                                registry.add_async_meta_method_mut(#meta_name, #closure_params #body);
                            },
                            _ => quote! {
                                registry.add_async_meta_method(#meta_name, #closure_params #body);
                            },
                        };
                        registration_calls.push(with_cfg(tokens, &method.attrs));
                    } else {
                        let meta_name = match lua_attr.meta_name(fn_name) {
                            Ok(name) => name,
                            Err(err) => return err.to_compile_error().into(),
                        };
                        let closure_params = gen_closure_params(&info);
                        let call_args = gen_call_args(&info);
                        let fn_path = quote! { #type_path::#fn_name };

                        let body = if lua_attr.infallible {
                            quote! { Ok(#fn_path(#call_args)) }
                        } else {
                            quote! { #fn_path(#call_args) }
                        };

                        let tokens = match info.instance {
                            Instance::None => quote! {
                                registry.add_meta_function(#meta_name, #closure_params { #body });
                            },
                            Instance::Ref(RefKind::Mut) => quote! {
                                registry.add_meta_method_mut(#meta_name, #closure_params { #body });
                            },
                            _ => quote! {
                                registry.add_meta_method(#meta_name, #closure_params { #body });
                            },
                        };

                        registration_calls.push(with_cfg(tokens, &method.attrs));
                    }

                    continue;
                }

                if is_async {
                    let fn_path = quote! { #type_path::#fn_name };
                    let closure_params = gen_async_closure_params(&info);
                    let call_args = gen_async_call_args(&info);
                    let lua_name = lua_attr.name_or_default(fn_name);

                    let body = if lua_attr.infallible {
                        quote! { async move { Ok(#fn_path(#call_args).await) } }
                    } else {
                        quote! { async move { #fn_path(#call_args).await } }
                    };
                    let tokens = match info.instance {
                        Instance::Ref(RefKind::Mut | RefKind::OptionMut) => quote! {
                            registry.add_async_method_mut(#lua_name, #closure_params #body);
                        },
                        Instance::Ref(_) => quote! {
                            registry.add_async_method(#lua_name, #closure_params #body);
                        },
                        Instance::Owned => quote! {
                            registry.add_async_method_once(#lua_name, #closure_params #body);
                        },
                        Instance::None => quote! {
                            registry.add_async_function(#lua_name, #closure_params #body);
                        },
                    };
                    registration_calls.push(with_cfg(tokens, &method.attrs));
                } else {
                    let fn_path = quote! { #type_path::#fn_name };
                    let closure_params = gen_closure_params(&info);
                    let call_args = gen_call_args(&info);
                    let lua_name = lua_attr.name_or_default(fn_name);

                    let body = if lua_attr.infallible {
                        quote! { Ok(#fn_path(#call_args)) }
                    } else {
                        quote! { #fn_path(#call_args) }
                    };
                    let tokens = match info.instance {
                        Instance::Ref(RefKind::Mut | RefKind::OptionMut) => quote! {
                            registry.add_method_mut(#lua_name, #closure_params { #body });
                        },
                        Instance::Ref(_) => quote! {
                            registry.add_method(#lua_name, #closure_params { #body });
                        },
                        Instance::Owned => quote! {
                            registry.add_method_once(#lua_name, #closure_params { #body });
                        },
                        Instance::None => quote! {
                            registry.add_function(#lua_name, #closure_params { #body });
                        },
                    };
                    registration_calls.push(with_cfg(tokens, &method.attrs));
                }
            }
            _ => {}
        }
    }

    for item in &mut input.items {
        match item {
            ImplItem::Const(c) => c.attrs = strip_lua_attrs(&c.attrs),
            ImplItem::Fn(m) => m.attrs = strip_lua_attrs(&m.attrs),
            _ => {}
        }
    }
    input.attrs = strip_lua_attrs(&input.attrs);

    quote! {
        #[allow(non_snake_case)]
        fn #register_fn_name<T: ::mlua_extras::typed::TypedDataFields<#type_path> + ::mlua_extras::typed::TypedDataMethods<#type_path>>(registry: &mut T) {
            use ::mlua_extras::typed::{TypedDataFields as _, TypedDataMethods as _};
            #(#registration_calls)*
        }

        #[allow(non_snake_case)]
        fn #register_fn_name_unwrapped(registry: &mut ::mlua_extras::typed::TypedUserDataRegistry<#type_path>) {
            #register_fn_name(registry);
        }

        #[allow(non_snake_case)]
        fn #register_fn_name_wrapped<'ctx>(registry: &mut ::mlua_extras::typed::registry::wrapper::TypedUserDataRegistry<'ctx, ::mlua_extras::mlua::userdata::UserDataRegistry<#type_path>>) {
            #register_fn_name(registry);
        }

        ::mlua::__inventory::submit! {
            #registration_type_name {
                register: #register_fn_name_unwrapped,
                register_wrapped: #register_fn_name_wrapped,
            }
        }

        #input
    }
}

/// Generate the call-site expression for a single argument.
fn gen_arg_token(arg: &ArgInfo) -> TokenStream {
    let ident = &arg.ident;
    match arg.userdata_ref {
        Some(RefKind::Ref) => quote! { &*#ident },
        Some(RefKind::Mut) => quote! { &mut *#ident },
        Some(RefKind::OptionRef) => quote! { #ident.as_ref().map(|r| &**r) },
        Some(RefKind::OptionMut) => quote! { #ident.as_mut().map(|r| &mut **r) },
        None => quote! { #ident },
    }
}

fn gen_arg_metadata(calls: &mut Vec<TokenStream>, arg: &ArgInfo) {
    let name = arg.ident.to_string();
    let docs = match arg.docs.as_ref() {
        Some(docs) => quote! { #docs },
        None => quote! { () },
    };

    if arg.variadic {
        calls.push(
            quote! { ::mlua_extras::typed::TypedDataMethods::param(registry, "...", #docs); },
        );
    } else {
        calls.push(
            quote! { ::mlua_extras::typed::TypedDataMethods::param(registry, #name, #docs); },
        );
    }
}

/// Generate call arguments for invoking the original method.
fn gen_call_args(info: &MethodInfo) -> TokenStream {
    let mut call_args: Vec<TokenStream> = Vec::new();
    let this = Ident::new("this", Span::mixed_site());
    let lua = Ident::new("lua", Span::mixed_site());

    match info.instance {
        Instance::None => {}
        _ => call_args.push(quote! { #this }),
    }

    if info.lua.is_some() {
        call_args.push(quote! { #lua });
    }

    for arg in &info.args {
        call_args.push(gen_arg_token(arg));
    }

    quote! { #(#call_args),* }
}

/// Generate the closure argument destructuring pattern.
fn gen_closure_destructure(info: &MethodInfo) -> TokenStream {
    if info.args.is_empty() {
        return quote! { () };
    }
    let idents: Vec<_> = (info.args)
        .iter()
        .map(|a| {
            let ident = &a.ident;
            if matches!(a.userdata_ref, Some(RefKind::Mut | RefKind::OptionMut)) {
                quote! { mut #ident }
            } else {
                quote! { #ident }
            }
        })
        .collect();
    let types: Vec<_> = info.args.iter().map(|a| &a.callback_type).collect();
    quote! { (#(#idents),*): (#(#types),*) }
}

/// Generate the closure params for the registration callback.
fn gen_closure_params(info: &MethodInfo) -> TokenStream {
    let destructure = gen_closure_destructure(info);
    let this = Ident::new("this", Span::mixed_site());
    let lua = Ident::new("lua", Span::mixed_site());

    match info.instance {
        Instance::None => quote! { |#lua, #destructure| },
        _ => quote! { |#lua, #this, #destructure| },
    }
}

/// Generate the closure params for an async registration callback.
fn gen_async_closure_params(info: &MethodInfo) -> TokenStream {
    let destructure = gen_closure_destructure(info);
    let this = Ident::new("this", Span::mixed_site());
    let lua = Ident::new("lua", Span::mixed_site());

    match info.instance {
        Instance::None => quote! { |#lua, #destructure| },
        Instance::Ref(RefKind::Mut) => quote! { |#lua, mut #this, #destructure| },
        _ => quote! { |#lua, #this, #destructure| },
    }
}

/// Generate call arguments for invoking the original async method.
fn gen_async_call_args(info: &MethodInfo) -> TokenStream {
    let mut call_args: Vec<TokenStream> = Vec::new();
    let this = Ident::new("this", Span::mixed_site());
    let lua = Ident::new("lua", Span::mixed_site());

    match info.instance {
        Instance::None => {}
        Instance::Ref(RefKind::Mut | RefKind::OptionMut) => call_args.push(quote! { &mut #this }),
        Instance::Ref(_) => call_args.push(quote! { &#this }),
        Instance::Owned => call_args.push(quote! { #this }),
    }

    match info.lua {
        Some(Lua::Ref) => call_args.push(quote! { &#lua }),
        Some(Lua::Owned) => call_args.push(quote! { #lua }),
        None => {}
    }

    for arg in &info.args {
        call_args.push(gen_arg_token(arg));
    }

    quote! { #(#call_args),* }
}
