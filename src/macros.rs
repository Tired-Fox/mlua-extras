#[macro_export]
macro_rules! function {
    {
        $(#[$($attr: tt)*])*
        $lua: ident fn $name: ident(
            $l: ident $(: $lty: ty)?
            $(, $arg: ident : $aty: ty)*
        ) $(-> $ret: ty)? {
            $($body: tt)*
        }
    } => {
        $lua.create_function(|$l $(: $lty)?, ($($arg,)*): ($($aty,)*)| $(-> $ret)? {
            $($body)*
        })
    };
    {
        $(#[$($attr: tt)*])*
        $lua: ident fn $source: ident . $name: ident(
            $l: ident $(: $lty: ty)?
            $(, $arg: ident : $aty: ty)*
        ) $(-> $ret: ty)? {
            $($body: tt)*
        }
    } => {
        match $lua.create_function(|$l$(: $lty)?, ($($arg,)*): ($($aty,)*)| $(-> $ret)? {
            $($body)*
        }) {
            Ok(fun) => $source.set(stringify!($name), fun),
            Err(err) => Err(err)
        }
    };
    {
        $(#[$($attr: tt)*])*
        $lua: ident fn $source: ident $(::$inner: ident)* . $name: ident(
            $l: ident $(: $lty: ty)?
            $(, $arg: ident : $aty: ty)*
        ) $(-> $ret: ty)? {
            $($body: tt)*
        }
    } => {
        {
            match $source.require::<mlua::Table>(stringify!($($inner.)*)) $(-> $ret: ty)? {
                Ok(table) => match $lua.create_function(|$l$(: $lty)?, ($($arg,)*): ($($aty,)*)| $(-> $ret)? {
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
