// The `aligned_borrow_derive` macro is taken from valida-xyz/valida under MIT license
//
// The MIT License (MIT)
//
// Copyright (c) 2023 The Valida Authors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::parse_quote;
use syn::Data;
use syn::DeriveInput;
use syn::GenericParam;
use syn::ItemFn;

#[proc_macro_derive(AlignedBorrow)]
pub fn aligned_borrow_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // Ensure the first generic parameter is the type generic, and rest all are const generics.
    let mut generics_iter = ast.generics.params.iter();

    // Extract the first generic parameter and ensure it's a type.
    let type_generic = match generics_iter.next().expect("No generic parameters found") {
        GenericParam::Type(type_param) => &type_param.ident,
        _ => panic!("The first generic parameter must be a type."),
    };

    // Collect the remaining generic parameters, ensuring they are all const generics.
    let const_generics: Vec<_> = generics_iter.map(|param| match param {
    GenericParam::Const(const_param) => &const_param.ident,
        _ => panic!("`AlignedBorrow` supports only a type as the first generic parameter and const generics after that"),
    }).collect();

    let methods = {
        quote! {
            impl<#type_generic: Copy #(, const #const_generics: usize)*> core::borrow::Borrow<#name<#type_generic #(, #const_generics)*>> for [#type_generic] {
                fn borrow(&self) -> &#name<#type_generic #(, #const_generics)*> {
                    debug_assert_eq!(self.len(), std::mem::size_of::<#name<u8 #(, #const_generics)*>>());
                    let (prefix, shorts, _suffix) = unsafe { self.align_to::<#name<#type_generic #(, #const_generics)*>>() };
                    debug_assert!(prefix.is_empty(), "Alignment should match");
                    debug_assert_eq!(shorts.len(), 1);
                    &shorts[0]
                }
            }

            impl<#type_generic: Copy #(, const #const_generics: usize)*> core::borrow::BorrowMut<#name<#type_generic #(, #const_generics)*>> for [#type_generic] {
                fn borrow_mut(&mut self) -> &mut #name<#type_generic #(, #const_generics)*> {
                    debug_assert_eq!(self.len(), std::mem::size_of::<#name<u8 #(, #const_generics)*>>());
                    let (prefix, shorts, _suffix) = unsafe { self.align_to_mut::<#name<#type_generic #(, #const_generics)*>>() };
                    debug_assert!(prefix.is_empty(), "Alignment should match");
                    debug_assert_eq!(shorts.len(), 1);
                    &mut shorts[0]
                }
            }
        }
    };

    TokenStream::from(methods)
}

#[proc_macro_derive(MachineAir, attributes(wp1_core_path, execution_record_path))]
pub fn machine_air_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generics = &ast.generics;
    let wp1_core_path = find_wp1_core_path(&ast.attrs);
    let execution_record_path = find_execution_record_path(&ast.attrs);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    match &ast.data {
        Data::Struct(_) => unimplemented!("Structs are not supported yet"),
        Data::Enum(e) => {
            let variants = e
                .variants
                .iter()
                .map(|variant| {
                    let variant_name = &variant.ident;

                    let mut fields = variant.fields.iter();
                    let field = fields.next().unwrap();
                    assert!(fields.next().is_none(), "Only one field is supported");
                    (variant_name, field)
                })
                .collect::<Vec<_>>();

            let width_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as p3_air::BaseAir<F>>::width(x)
                }
            });

            let base_air = quote! {
                impl #impl_generics p3_air::BaseAir<F> for #name #ty_generics #where_clause {
                    fn width(&self) -> usize {
                        match self {
                            #(#width_arms,)*
                        }
                    }

                    fn preprocessed_trace(&self) -> Option<p3_matrix::dense::RowMajorMatrix<F>> {
                        unreachable!("A machine air should use the preprocessed trace from the `MachineAir` trait")
                    }
                }
            };

            let name_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as #wp1_core_path::air::MachineAir<F>>::name(x)
                }
            });

            let preprocessed_width_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as #wp1_core_path::air::MachineAir<F>>::preprocessed_width(x)
                }
            });

            let generate_preprocessed_trace_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as #wp1_core_path::air::MachineAir<F>>::generate_preprocessed_trace(x, program)
                }
            });

            let generate_trace_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as #wp1_core_path::air::MachineAir<F>>::generate_trace(x, input, output)
                }
            });

            let generate_dependencies_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as #wp1_core_path::air::MachineAir<F>>::generate_dependencies(x, input, output)
                }
            });

            let included_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as #wp1_core_path::air::MachineAir<F>>::included(x, shard)
                }
            });

            let machine_air = quote! {
                impl #impl_generics #wp1_core_path::air::MachineAir<F> for #name #ty_generics #where_clause {
                    type Record = #execution_record_path;

                    fn name(&self) -> String {
                        match self {
                            #(#name_arms,)*
                        }
                    }

                    fn preprocessed_width(&self) -> usize {
                        match self {
                            #(#preprocessed_width_arms,)*
                        }
                    }

                    fn generate_preprocessed_trace(
                        &self,
                        program: &#wp1_core_path::runtime::Program,
                    ) -> Option<p3_matrix::dense::RowMajorMatrix<F>> {
                        match self {
                            #(#generate_preprocessed_trace_arms,)*
                        }
                    }

                    fn generate_trace(
                        &self,
                        input: &#execution_record_path,
                        output: &mut #execution_record_path,
                    ) -> p3_matrix::dense::RowMajorMatrix<F> {
                        match self {
                            #(#generate_trace_arms,)*
                        }
                    }

                    fn generate_dependencies(
                        &self,
                        input: &#execution_record_path,
                        output: &mut #execution_record_path,
                    ) {
                        match self {
                            #(#generate_dependencies_arms,)*
                        }
                    }

                    fn included(&self, shard: &Self::Record) -> bool {
                        match self {
                            #(#included_arms,)*
                        }
                    }
                }
            };

            let eval_arms = variants.iter().map(|(variant_name, field)| {
                let field_ty = &field.ty;
                quote! {
                    #name::#variant_name(x) => <#field_ty as p3_air::Air<AB>>::eval(x, builder)
                }
            });

            // Attach an extra generic AB : crate::air::SP1AirBuilder to the generics of the enum
            let generics = &ast.generics;
            let mut new_generics = generics.clone();
            new_generics
                .params
                .push(syn::parse_quote! { AB: #wp1_core_path::air::SP1AirBuilder<F = F> });

            let (air_impl_generics, _, _) = new_generics.split_for_impl();

            let air = quote! {
                impl #air_impl_generics p3_air::Air<AB> for #name #ty_generics #where_clause {
                    fn eval(&self, builder: &mut AB) {
                        match self {
                            #(#eval_arms,)*
                        }
                    }
                }
            };

            quote! {
                #base_air

                #machine_air

                #air
            }
            .into()
        }
        Data::Union(_) => unimplemented!("Unions are not supported"),
    }
}

#[proc_macro_attribute]
pub fn cycle_tracker(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let visibility = &input.vis;
    let name = &input.sig.ident;
    let inputs = &input.sig.inputs;
    let output = &input.sig.output;
    let block = &input.block;
    let generics = &input.sig.generics;
    let where_clause = &input.sig.generics.where_clause;

    let result = quote! {
        #visibility fn #name #generics (#inputs) #output #where_clause {
            println!("cycle-tracker-start: {}", stringify!(#name));
            let result = (|| #block)();
            println!("cycle-tracker-end: {}", stringify!(#name));
            result
        }
    };

    result.into()
}

fn find_wp1_core_path(attrs: &[syn::Attribute]) -> syn::Ident {
    for attr in attrs {
        if attr.path.is_ident("wp1_core_path") {
            if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
                if let syn::Lit::Str(lit_str) = &meta.lit {
                    return syn::Ident::new(&lit_str.value(), lit_str.span());
                }
            }
        }
    }
    syn::Ident::new("crate", proc_macro2::Span::call_site())
}

fn find_execution_record_path(attrs: &[syn::Attribute]) -> syn::Path {
    for attr in attrs {
        if attr.path.is_ident("execution_record_path") {
            if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
                if let syn::Lit::Str(lit_str) = &meta.lit {
                    if let Ok(path) = lit_str.parse::<syn::Path>() {
                        return path;
                    }
                }
            }
        }
    }
    parse_quote!(crate::runtime::ExecutionRecord)
}
