#![allow(non_snake_case, clippy::missing_safety_doc)]

#[macro_export]
macro_rules! generate_proxy {
    (
        @generate_alias
        alias: $alias:ident,
        target_fn: $fname:ident,
        args: ( [ $( ($arg:ident : $arg_ty:ty) ),* ] ),
        ret: $ret:ty
    ) => {
        paste::paste! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $alias( $( $arg : $arg_ty ),* ) -> $ret {
                let f = *[<__REAL_ $fname:upper>];
                f( $( $arg ),* )
            }
        }
    };

    // ─── 2. Iterator: Recurse Aliases ────────────────────────────────────────
    // This rule iterates over the aliases. Crucially, it takes `args_tt:tt`.
    // Because `args_tt` is a single token tree (not a repetition variable),
    // it does not conflict with the `$alias` repetition loop.
    (
        @recurse_aliases
        target_fn: $fname:ident,
        ret: $ret:ty,
        args_tt: $args_tt:tt,
        aliases: [ $($alias:ident),* ]
    ) => {
        $(
            $crate::generate_proxy!(
                @generate_alias
                alias: $alias,
                target_fn: $fname,
                args: $args_tt, // Pass the opaque blob to the leaf
                ret: $ret
            );
        )*
    };

    // ─── 3. Helper: Generate Main Function ───────────────────────────────────
    // Standard generation for the primary function.
    (
        @generate_main
        fn $fname:ident ( [ $( ($arg:ident : $arg_ty:ty) ),* ] ) -> $ret:ty;
        target_symbol: $real_sym:ident
    ) => {
        paste::paste! {
            static [<__REAL_ $fname:upper>]: ::once_cell::sync::Lazy<
                 extern "C" fn( $($arg_ty),* ) -> $ret
            > = ::once_cell::sync::Lazy::new(|| {
                let name = concat!(stringify!($real_sym), "\0");
                let ptr = $crate::dlsym_next(name.as_bytes());
                if ptr.is_null() {
                    eprintln!("fatal: symbol '{}' not found in underlying libcuda", name);
                    std::process::abort();
                }
                unsafe { std::mem::transmute(ptr) }
            });

            #[unsafe(no_mangle)]
            pub extern "C" fn $fname( $( $arg : $arg_ty ),* ) -> $ret {
                let f = *[<__REAL_ $fname:upper>];
                f( $( $arg ),* )
            }
        }
    };

    // ─── 4. Main Entry Point ─────────────────────────────────────────────────
    // Matches `fn Name ([...])`. The `args_tt` captures `([ ... ])` including
    // the surrounding parentheses.
    (
        fn $fname:ident $args_tt:tt -> $ret:ty;
        name: $real_sym:ident
        $(, aliases: $($alias:ident),* )?
    ) => {
        // 1. Generate the main function
        $crate::generate_proxy!(
            @generate_main
            fn $fname $args_tt -> $ret;
            target_symbol: $real_sym
        );

        // 2. Generate aliases (if any).
        // We pass the opaque `args_tt` to the recursion helper.
        $(
            $crate::generate_proxy!(
                @recurse_aliases
                target_fn: $fname,
                ret: $ret,
                args_tt: $args_tt,
                aliases: [ $($alias),* ]
            );
        )?
    };
}

pub mod driver {
    use cust_raw::driver_internal_sys::*;
    include!(concat!(env!("OUT_DIR"), "/passthroughs_driver.rs"));
}
