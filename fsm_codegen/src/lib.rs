#![recursion_limit = "256"]

extern crate proc_macro;
extern crate fsm;

extern crate petgraph;

use proc_macro::TokenStream;

extern crate syn;

#[macro_use]
extern crate quote;


mod codegen;
mod fsm_def;
mod parse;
mod viz;
mod graph;

use codegen::*;
use parse::*;
use viz::*;



#[proc_macro_derive(Fsm)]
pub fn derive_fsm(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();

    let desc = parse_description(&ast);

    //panic!("fsm: {:?}", fsm);

    let enums = build_enums(&desc);
    let main = build_main_struct(&desc);
    let state_store = build_state_store(&desc);

    let viz_test = build_test_viz_build(&desc);

    let q = quote! {
        #enums
        #state_store
        #main

        #viz_test
    };

    //panic!("q: {:?}", q.to_string());

    //let q = q.to_string().parse().unwrap();
    //q

    q.to_string().parse().unwrap()

    //quote!(#fsm).to_string().parse().unwrap()
}

