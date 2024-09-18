#[macro_export]
macro_rules! function {
    {
        #[$lua: ident]
        fn $name: ident($l: ident $(: $lty: ty)?) {
            $($body: tt)*
        }
    } => {
        $lua.create_function(|$l$(: $lty)?, _: ()| {
            $($body)*
        })
    };
    {
        #[$lua: ident]
        fn $source: ident $(::$inner: ident)*  $name: ident(
            $l: ident $(: $lty: ty)?
            $(, $arg: ident : $aty: ty)*
        ) {
            $($body: tt)*
        }
    } => {
        {
            match $source.require::<mlua::Table>(stringify!($($inner.)*)) {
                Ok(table) => match $lua.create_function(|$l$(: $lty)?, ($($arg,)*): ($($aty,)*)| {
                    $($body)*
                }) {
                    Ok(fun) => table.set(stringify!($name), fun),
                    Err(err) => Err(err)
                }
                Err(err) => Err(err)
            }
        }
    };
}
