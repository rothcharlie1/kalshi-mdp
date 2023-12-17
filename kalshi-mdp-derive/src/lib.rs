use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(SetTimestamp)]
pub fn set_timestamp_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Check if the struct has a field named 'ts'
    let has_ts_field = if let Data::Struct(data_struct) = &input.data {
        data_struct.fields.iter().any(|field| field.ident.as_ref().map_or(false, |ident| ident == "ts"))
    } else {
        false
    };

    if has_ts_field {
        // Generate the implementation of the SetTimestamp trait
        let gen = quote! {
            impl SetTimestamp for #struct_name {
                fn set_timestamp(&self) -> Self {
                    let ts = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("time went backwards")
                            .as_nanos() as u64);
                    let old = self.clone();
                    #struct_name {
                        ts: ts,
                        ..old
                    }
                }
            }
        };
        gen.into()
    } else {
        // If the struct does not have a 'ts' field, generate an empty implementation
        let gen = quote! {
            impl SetTimestamp for #struct_name {
                fn set_timestamp(&self) -> Self {
                    self
                }
            }
        };
        gen.into()
    }
}
