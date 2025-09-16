//! Procedural macros for the tagged_dispatch crate.
//!
//! This crate provides the `#[tagged_dispatch]` attribute macro for both traits and enums,
//! enabling memory-efficient polymorphic dispatch using tagged pointers.

use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Data, DataEnum, DeriveInput, Fields,
    Ident, ItemTrait, Path, Result, Token, TraitItem, TraitItemFn,
    Type,
};
use heck::ToSnakeCase;
use proc_macro2::TokenStream as TokenStream2;

// Helper functions for conditional code generation based on features

/// Generate allocator match arms based on enabled features at macro build time
fn generate_allocator_arms(field_name: &Ident, ty: &Type, arena_type_name: &Ident) -> TokenStream2 {
    #[cfg(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo"))]
    let mut arms = vec![];

    #[cfg(not(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo")))]
    let arms: Vec<TokenStream2> = vec![];

    // Add typed-arena arm if feature is enabled at macro build time
    #[cfg(feature = "allocator-typed-arena")]
    arms.push(quote! {
        #arena_type_name::Typed { #field_name, .. } => {
            #field_name.alloc(value) as *mut #ty as *mut ()
        }
    });

    // Add bumpalo arm if feature is enabled at macro build time
    #[cfg(feature = "allocator-bumpalo")]
    arms.push(quote! {
        #arena_type_name::Bumpalo { arena, .. } => {
            unsafe {
                let arena_ref = &**arena;
                arena_ref.alloc(value) as *mut #ty as *mut ()
            }
        }
    });

    // If no allocators are enabled, generate a compile error
    if arms.is_empty() {
        let _ = (field_name, ty, arena_type_name); // Suppress unused warnings
        quote! {
            _ => compile_error!("At least one allocator feature must be enabled (allocator-typed-arena or allocator-bumpalo)")
        }
    } else {
        quote! { #(#arms)* }
    }
}

/// Generate arena enum definition based on enabled features
fn generate_arena_enum(arena_type_name: &Ident, lifetime: &TokenStream2, typed_arena_fields: &[TokenStream2]) -> TokenStream2 {
    #[cfg(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo"))]
    let mut variants = vec![];

    #[cfg(not(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo")))]
    let variants: Vec<TokenStream2> = vec![];

    #[cfg(feature = "allocator-typed-arena")]
    variants.push(quote! {
        Typed {
            #(#typed_arena_fields,)*
        }
    });

    #[cfg(feature = "allocator-bumpalo")]
    variants.push(quote! {
        Bumpalo {
            arena: *mut ::bumpalo::Bump,
            owned: bool,
            _phantom: ::core::marker::PhantomData<&#lifetime ()>,
        }
    });

    // If no variants, the enum would be empty - generate compile error
    if variants.is_empty() {
        let _ = typed_arena_fields; // Suppress unused warning
        quote! {
            compile_error!("At least one allocator feature must be enabled");
        }
    } else {
        quote! {
            /// Internal arena type enum
            #[doc(hidden)]
            enum #arena_type_name<#lifetime> {
                #(#variants,)*
            }
        }
    }
}

/// Generate builder constructor implementation based on enabled features
fn generate_builder_new() -> TokenStream2 {
    // Prefer bumpalo if available, fall back to typed-arena
    #[cfg(feature = "allocator-bumpalo")]
    return quote! {
        Self::with_bumpalo()
    };

    #[cfg(all(feature = "allocator-typed-arena", not(feature = "allocator-bumpalo")))]
    return quote! {
        Self::with_typed_arena()
    };

    #[cfg(not(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo")))]
    quote! {
        compile_error!("At least one allocator feature must be enabled (allocator-typed-arena or allocator-bumpalo)")
    }
}

/// Generate builder methods for specific allocators
fn generate_builder_methods(
    builder_name: &Ident,
    arena_type_name: &Ident,
    typed_arena_inits: &[TokenStream2],
    lifetime: &TokenStream2
) -> TokenStream2 {
    #[cfg(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo"))]
    let mut methods = vec![];

    #[cfg(not(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo")))]
    let methods: Vec<TokenStream2> = {
        let _ = (builder_name, arena_type_name, typed_arena_inits, lifetime); // Suppress unused warnings
        vec![]
    };

    #[cfg(feature = "allocator-bumpalo")]
    methods.push(quote! {
        /// Create a builder with owned bumpalo arena
        pub fn with_bumpalo() -> #builder_name<'static> {
            // Use a leaked Box to get 'static lifetime for owned arena
            let arena = Box::leak(Box::new(::bumpalo::Bump::new()));
            #builder_name {
                allocator: #arena_type_name::Bumpalo {
                    arena: arena as *mut _,
                    owned: true,
                    _phantom: ::core::marker::PhantomData,
                },
                _phantom: ::core::marker::PhantomData,
            }
        }

        /// Create a builder with external bumpalo arena
        pub fn with_external_bumpalo(arena: &#lifetime ::bumpalo::Bump) -> Self {
            Self {
                allocator: #arena_type_name::Bumpalo {
                    arena: arena as *const _ as *mut _,
                    owned: false,
                    _phantom: ::core::marker::PhantomData,
                },
                _phantom: ::core::marker::PhantomData,
            }
        }
    });

    #[cfg(feature = "allocator-typed-arena")]
    methods.push(quote! {
        /// Create a builder with typed arenas
        pub fn with_typed_arena() -> Self {
            Self {
                allocator: #arena_type_name::Typed {
                    #(#typed_arena_inits,)*
                },
                _phantom: ::core::marker::PhantomData,
            }
        }
    });

    quote! { #(#methods)* }
}

/// Generate reset implementation based on enabled features
fn generate_reset_impl(
    arena_type_name: &Ident,
    typed_arena_inits2: &[TokenStream2]
) -> TokenStream2 {
    #[cfg(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo"))]
    let mut arms = vec![];

    #[cfg(not(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo")))]
    let arms: Vec<TokenStream2> = {
        let _ = (arena_type_name, typed_arena_inits2); // Suppress unused warnings
        vec![]
    };

    #[cfg(feature = "allocator-typed-arena")]
    arms.push(quote! {
        #arena_type_name::Typed { .. } => {
            // typed_arena doesn't support reset, must create new arenas
            self.allocator = #arena_type_name::Typed {
                #(#typed_arena_inits2,)*
            };
        }
    });

    #[cfg(feature = "allocator-bumpalo")]
    arms.push(quote! {
        #arena_type_name::Bumpalo { arena, owned: true, .. } => {
            // SAFETY: We know this is safe because we own the arena
            unsafe {
                (&mut **arena).reset();
            }
        }
        #arena_type_name::Bumpalo { owned: false, .. } => {
            panic!("Cannot reset builder using external arena");
        }
    });

    quote! {
        match &mut self.allocator {
            #(#arms)*
        }
    }
}

/// Generate stats implementation based on enabled features
fn generate_stats_impl(arena_type_name: &Ident) -> TokenStream2 {
    #[cfg(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo"))]
    let mut arms = vec![];

    #[cfg(not(any(feature = "allocator-typed-arena", feature = "allocator-bumpalo")))]
    let arms: Vec<TokenStream2> = {
        let _ = arena_type_name; // Suppress unused warning
        vec![]
    };

    #[cfg(feature = "allocator-typed-arena")]
    arms.push(quote! {
        #arena_type_name::Typed { .. } => {
            // typed_arena doesn't expose statistics
            Default::default()
        }
    });

    #[cfg(feature = "allocator-bumpalo")]
    arms.push(quote! {
        #arena_type_name::Bumpalo { arena, .. } => {
            unsafe {
                let arena_ref = &**arena;
                ::tagged_dispatch::ArenaStats {
                    allocated_bytes: arena_ref.allocated_bytes(),
                    chunk_capacity: arena_ref.chunk_capacity(),
                }
            }
        }
    });

    quote! {
        match &self.allocator {
            #(#arms)*
        }
    }
}

/// Attribute macro for traits that will be used with tagged dispatch.
///
/// # Example
/// ```ignore
/// #[tagged_dispatch]
/// trait Draw {
///     fn draw(&self);
///
///     #[no_dispatch]
///     fn debug_name(&self) -> &str { "drawable" }
/// }
/// ```
#[proc_macro_attribute]
pub fn tagged_dispatch(args: TokenStream, input: TokenStream) -> TokenStream {
    // Check if this is being applied to a trait or an enum
    if let Ok(trait_def) = syn::parse::<ItemTrait>(input.clone()) {
        process_trait(trait_def)
    } else if let Ok(enum_def) = syn::parse::<DeriveInput>(input) {
        process_enum(args, enum_def)
    } else {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "tagged_dispatch can only be applied to traits or enums"
        )
        .to_compile_error()
        .into()
    }
}

/// Process a trait definition with #[tagged_dispatch]
fn process_trait(mut trait_def: ItemTrait) -> TokenStream {
    let trait_name = &trait_def.ident;
    
    // Extract methods that should be dispatched (those without #[no_dispatch])
    let dispatch_methods: Vec<_> = trait_def.items.iter().filter_map(|item| {
        if let TraitItem::Fn(method) = item {
            let has_no_dispatch = method.attrs.iter().any(|attr| 
                attr.path().is_ident("no_dispatch")
            );
            if !has_no_dispatch {
                Some(method.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).collect();
    
    // Clean the trait definition (remove no_dispatch attributes)
    for item in &mut trait_def.items {
        if let TraitItem::Fn(method) = item {
            method.attrs.retain(|attr| !attr.path().is_ident("no_dispatch"));
        }
    }
    
    // Generate the dispatch implementation macro name
    let macro_name = format_ident!("__impl_{}_dispatch", trait_name.to_string().to_snake_case());
    
    // Generate dispatch method implementations
    let dispatch_impls: Vec<_> = dispatch_methods.iter().map(|method| {
        generate_dispatch_method(method)
    }).collect();
    
    let output = quote! {
        // The original trait
        #trait_def
        
        // Hidden macro that implements dispatch for this trait
        #[doc(hidden)]
        macro_rules! #macro_name {
            (
                $enum_name:ident,
                $enum_type_name:ident,
                owned,
                [$(($variant:ident, $type:ty)),* $(,)?]
            ) => {
                impl $enum_name {
                    #(#dispatch_impls)*
                }
            };
            
            // Arena version with lifetime
            (
                $enum_name:ident,
                $enum_type_name:ident,
                $lifetime:lifetime,
                [$(($variant:ident, $type:ty)),* $(,)?]
            ) => {
                impl<$lifetime> $enum_name<$lifetime> {
                    #(#dispatch_impls)*
                }
            };
        }
    };
    
    TokenStream::from(output)
}

/// Process an enum definition with #[tagged_dispatch(Trait1, Trait2, ...)]
fn process_enum(args: TokenStream, mut enum_def: DeriveInput) -> TokenStream {
    // Parse the trait list
    let traits = parse_macro_input!(args as TraitList);
    
    let enum_name = &enum_def.ident;
    let vis = &enum_def.vis;
    let generics = &enum_def.generics;
    
    // Check if this is an arena version (has lifetime parameter)
    let has_lifetime = !generics.lifetimes().collect::<Vec<_>>().is_empty();
    let lifetime = generics.lifetimes().next().map(|lt| &lt.lifetime);
    
    // Transform enum variants to ensure they all have types
    let variants = if let Data::Enum(ref mut data_enum) = enum_def.data {
        process_enum_variants(data_enum)
    } else {
        return syn::Error::new_spanned(
            enum_def,
            "tagged_dispatch can only be applied to enums"
        )
        .to_compile_error()
        .into();
    };
    
    // Generate the implementation based on whether it's arena or owned
    if has_lifetime {
        generate_arena_impl(enum_name, vis, lifetime.unwrap(), &variants, &traits)
    } else {
        generate_owned_impl(enum_name, vis, &variants, &traits)
    }
}

/// Process enum variants, converting shorthand syntax to full syntax
fn process_enum_variants(data_enum: &mut DataEnum) -> Vec<(Ident, Type)> {
    data_enum.variants.iter_mut().map(|variant| {
        match &mut variant.fields {
            Fields::Unit => {
                // Shorthand: convert `Circle` to `Circle(Circle)`
                let type_name = &variant.ident;
                let type_path: Type = syn::parse_quote!(#type_name);
                
                // Update the variant to have the type
                variant.fields = Fields::Unnamed(syn::parse_quote!((#type_path)));
                
                (variant.ident.clone(), type_path)
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Already has a type: `Circle(SomeType)`
                let inner_type = fields.unnamed.first().unwrap().ty.clone();
                (variant.ident.clone(), inner_type)
            }
            _ => {
                panic!("Each variant must either be a unit variant (shorthand) or have exactly one unnamed field");
            }
        }
    }).collect()
}

/// Generate implementation for owned version (no lifetime)
fn generate_owned_impl(
    enum_name: &Ident,
    vis: &syn::Visibility,
    variants: &[(Ident, Type)],
    traits: &TraitList,
) -> TokenStream {
    let enum_type_name = format_ident!("{}Type", enum_name);
    
    // Generate variant constructors
    let constructors = variants.iter().enumerate().map(|(i, (variant, ty))| {
        let tag = i as u8;
        let method_name = format_ident!("{}", variant.to_string().to_snake_case());
        quote! {
            #[doc = concat!("Create a `", stringify!(#variant), "` variant")]
            #[inline]
            pub fn #method_name(value: #ty) -> Self {
                let boxed = Box::new(value);
                let ptr = Box::into_raw(boxed) as *mut ();
                Self(::tagged_dispatch::TaggedPtr::new(ptr, #tag))
            }
        }
    });
    
    // Generate From implementations
    let from_impls = variants.iter().enumerate().map(|(i, (_variant, ty))| {
        let tag = i as u8;
        quote! {
            impl From<#ty> for #enum_name {
                fn from(value: #ty) -> Self {
                    let boxed = Box::new(value);
                    let ptr = Box::into_raw(boxed) as *mut ();
                    Self(::tagged_dispatch::TaggedPtr::new(ptr, #tag))
                }
            }
        }
    });
    
    // Generate Drop implementation
    let drop_arms = variants.iter().enumerate().map(|(i, (_variant, ty))| {
        let tag = i as u8;
        quote! {
            #tag => {
                // Use untagged_ptr() for deallocation to ensure we pass
                // the original pointer to Box::from_raw
                let ptr = self.0.untagged_ptr() as *mut #ty;
                drop(Box::from_raw(ptr));
            }
        }
    });
    
    // Generate Clone implementation
    let clone_arms = variants.iter().enumerate().map(|(i, (variant, ty))| {
        let method_name = format_ident!("{}", variant.to_string().to_snake_case());
        let tag = i as u8;
        quote! {
            #tag => {
                // Use ptr() which benefits from TBI on supported platforms
                let ptr = self.0.ptr() as *const #ty;
                let cloned = (*ptr).clone();
                Self::#method_name(cloned)
            }
        }
    });
    
    // Generate enum variants
    let enum_variants = variants.iter().map(|(variant, _)| {
        quote! { #variant }
    });
    
    // Generate variant list for dispatch macros
    let variant_list: Vec<_> = variants.iter().map(|(variant, ty)| {
        quote! { (#variant, #ty) }
    }).collect();

    // Generate dispatch macro invocations for each trait
    let dispatch_invocations = traits.items.iter().map(|trait_path| {
        let trait_name = &trait_path.segments.last().unwrap().ident;
        let macro_name = format_ident!("__impl_{}_dispatch", trait_name.to_string().to_snake_case());
        let variant_list = variant_list.clone();

        quote! {
            #macro_name!(#enum_name, #enum_type_name, owned, [#(#variant_list),*]);
        }
    });
    
    // Generate compile-time trait checks
    let trait_checks = traits.items.iter().flat_map(|trait_path| {
        variants.iter().map(move |(_, ty)| {
            quote! {
                const _: fn() = || {
                    fn assert_impl<T: #trait_path>() {}
                    assert_impl::<#ty>();
                };
            }
        })
    });
    
    let output = quote! {
        /// Tagged pointer dispatch type - only 8 bytes!
        #[repr(transparent)]
        #vis struct #enum_name(::tagged_dispatch::TaggedPtr<()>);
        
        /// Type variants for compile-time checking
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #vis enum #enum_type_name {
            #(#enum_variants,)*
        }
        
        impl #enum_name {
            #(#constructors)*
            
            #[inline(always)]
            pub fn tag_type(&self) -> #enum_type_name {
                unsafe { ::core::mem::transmute(self.0.tag()) }
            }
        }
        
        #(#from_impls)*
        
        impl Drop for #enum_name {
            fn drop(&mut self) {
                if self.0.is_null() {
                    return;
                }
                
                unsafe {
                    match self.0.tag() {
                        #(#drop_arms)*
                        _ => unreachable!("Invalid tag"),
                    }
                }
            }
        }
        
        impl Clone for #enum_name {
            fn clone(&self) -> Self {
                unsafe {
                    match self.0.tag() {
                        #(#clone_arms)*
                        _ => unreachable!("Invalid tag"),
                    }
                }
            }
        }
        
        // Apply dispatch implementations for each trait
        #(#dispatch_invocations)*
        
        // Compile-time trait implementation checks
        #(#trait_checks)*
        
        // Size assertion
        const _: () = assert!(::core::mem::size_of::<#enum_name>() == 8);
    };
    
    TokenStream::from(output)
}

/// Generate implementation for arena version (has lifetime)
fn generate_arena_impl(
    enum_name: &Ident,
    vis: &syn::Visibility,
    lifetime: &syn::Lifetime,
    variants: &[(Ident, Type)],
    traits: &TraitList,
) -> TokenStream {
    let enum_type_name = format_ident!("{}Type", enum_name);
    let builder_name = format_ident!("{}ArenaBuilder", enum_name);
    let arena_type_name = format_ident!("{}ArenaType", enum_name);

    // Generate typed arena field declarations for each variant
    let typed_arena_fields: Vec<_> = variants.iter().map(|(variant, ty)| {
        let field_name = format_ident!("{}_arena", variant.to_string().to_snake_case());
        quote! { #field_name: ::typed_arena::Arena<#ty> }
    }).collect();

    // Generate typed arena field initializations
    let typed_arena_inits: Vec<_> = variants.iter().map(|(variant, _ty)| {
        let field_name = format_ident!("{}_arena", variant.to_string().to_snake_case());
        quote! { #field_name: ::typed_arena::Arena::new() }
    }).collect();

    // Clone for second usage in reset
    let typed_arena_inits2 = typed_arena_inits.clone();

    // Generate builder methods for each variant
    let builder_methods = variants.iter().enumerate().map(|(i, (variant, ty))| {
        let tag = i as u8;
        let method_name = format_ident!("{}", variant.to_string().to_snake_case());
        let field_name = format_ident!("{}_arena", variant.to_string().to_snake_case());

        // Generate allocator match arms based on enabled features at macro build time
        let allocator_arms = generate_allocator_arms(&field_name, ty, &arena_type_name);

        quote! {
            #[doc = concat!("Create a `", stringify!(#variant), "` variant in the arena")]
            #[inline]
            pub fn #method_name(&#lifetime self, value: #ty) -> #enum_name<#lifetime> {
                let ptr = match &self.allocator {
                    #allocator_arms
                };

                #enum_name(::tagged_dispatch::TaggedPtr::new(ptr, #tag), ::core::marker::PhantomData)
            }
        }
    });

    // Generate enum variants
    let enum_variants = variants.iter().map(|(variant, _)| {
        quote! { #variant }
    });

    // Generate variant list for dispatch macros
    let variant_list: Vec<_> = variants.iter().map(|(variant, ty)| {
        quote! { (#variant, #ty) }
    }).collect();

    // Generate dispatch macro invocations for each trait
    let dispatch_invocations = traits.items.iter().map(|trait_path| {
        let trait_name = &trait_path.segments.last().unwrap().ident;
        let macro_name = format_ident!("__impl_{}_dispatch", trait_name.to_string().to_snake_case());
        let variant_list = variant_list.clone();

        quote! {
            #macro_name!(#enum_name, #enum_type_name, #lifetime, [#(#variant_list),*]);
        }
    });

    // Generate compile-time trait checks
    let trait_checks = traits.items.iter().flat_map(|trait_path| {
        variants.iter().map(move |(_, ty)| {
            quote! {
                const _: fn() = || {
                    fn assert_impl<T: #trait_path>() {}
                    assert_impl::<#ty>();
                };
            }
        })
    });

    // Generate the arena enum definition based on enabled features
    // Convert lifetime to TokenStream2
    let lifetime_tokens = quote! { #lifetime };
    let arena_enum_definition = generate_arena_enum(&arena_type_name, &lifetime_tokens, &typed_arena_fields);

    // Generate builder new implementation
    let builder_new_impl = generate_builder_new();

    // Generate builder methods
    let builder_specific_methods = generate_builder_methods(&builder_name, &arena_type_name, &typed_arena_inits, &lifetime_tokens);

    // Generate reset implementation
    let reset_impl = generate_reset_impl(&arena_type_name, &typed_arena_inits2);

    // Generate stats implementation
    let stats_impl = generate_stats_impl(&arena_type_name);

    let output = quote! {
        /// Arena-allocated tagged pointer dispatch type - only 8 bytes and Copy!
        #[repr(transparent)]
        #vis struct #enum_name<#lifetime>(
            ::tagged_dispatch::TaggedPtr<()>,
            ::core::marker::PhantomData<&#lifetime ()>
        );

        /// Type variants for compile-time checking
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #vis enum #enum_type_name {
            #(#enum_variants,)*
        }

        // Generate arena type enum based on enabled features at macro build time
        #arena_enum_definition

        /// Arena builder for creating arena-allocated variants
        #vis struct #builder_name<#lifetime> {
            allocator: #arena_type_name<#lifetime>,
            _phantom: ::core::marker::PhantomData<&#lifetime ()>,
        }

        impl<#lifetime> #builder_name<#lifetime> {
            /// Create a new builder with the default allocator
            /// (prefers bumpalo if available)
            pub fn new() -> Self {
                #builder_new_impl
            }

            #builder_specific_methods

            /// Reset all allocations
            pub fn reset(&mut self) {
                #reset_impl
            }

            /// Clear allocations and reclaim memory
            pub fn clear(&mut self) {
                self.reset(); // For now, same as reset
            }

            /// Get memory usage statistics
            pub fn stats(&self) -> ::tagged_dispatch::ArenaStats {
                #stats_impl
            }

            #(#builder_methods)*
        }

        impl<#lifetime> #enum_name<#lifetime> {
            /// Create a new arena builder for this type
            pub fn arena_builder() -> #builder_name<#lifetime> {
                #builder_name::new()
            }

            #[inline(always)]
            pub fn tag_type(&self) -> #enum_type_name {
                unsafe { ::core::mem::transmute(self.0.tag()) }
            }
        }

        // Arena version is Copy
        impl<#lifetime> Copy for #enum_name<#lifetime> {}

        impl<#lifetime> Clone for #enum_name<#lifetime> {
            #[inline(always)]
            fn clone(&self) -> Self {
                *self
            }
        }

        // No Drop impl needed - arena handles deallocation

        // Apply dispatch implementations for each trait
        #(#dispatch_invocations)*

        // Compile-time trait implementation checks
        #(#trait_checks)*

        // Size assertion
        const _: () = assert!(::core::mem::size_of::<#enum_name<'static>>() == 8);
    };

    TokenStream::from(output)
}

/// Generate a single dispatch method implementation
fn generate_dispatch_method(method: &TraitItemFn) -> proc_macro2::TokenStream {
    let method_name = &method.sig.ident;
    let inputs = &method.sig.inputs;
    let output = &method.sig.output;
    
    // Extract arguments (skip &self)
    let args: Vec<_> = inputs.iter().skip(1).collect();
    let arg_names: Vec<_> = args.iter().filter_map(|arg| {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                Some(&pat_ident.ident)
            } else {
                None
            }
        } else {
            None
        }
    }).collect();
    
    quote! {
        #[inline]
        pub fn #method_name(&self #(, #args)*) #output {
            unsafe {
                match self.tag_type() {
                    $(
                        $enum_type_name::$variant => {
                            let ptr = &*(self.0.ptr() as *const $type);
                            ptr.#method_name(#(#arg_names),*)
                        }
                    )*
                }
            }
        }
    }
}

/// Parser for comma-separated trait list
struct TraitList {
    items: Vec<Path>,
}

impl Parse for TraitList {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            // No traits specified, return empty list
            return Ok(TraitList { items: vec![] });
        }
        
        let items = Punctuated::<Path, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect();
        Ok(TraitList { items })
    }
}

