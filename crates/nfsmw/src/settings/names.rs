//! `names!`: the strict `FromStr` and `Display` of a setting that is a list of fixed lower-case names.

/// Implements `FromStr` (exact names only, anything else is an error saying what was expected) and `Display`
/// (the same names) for a fieldless enum.
macro_rules! names {
    ($ty:ty, $expected:literal, [$(($variant:path, $name:literal)),+ $(,)?]) => {
        impl ::std::str::FromStr for $ty {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, String> {
                match s {
                    $($name => Ok($variant),)+
                    _ => Err(format!("expected {}, got {s:?}", $expected)),
                }
            }
        }

        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(match self {
                    $($variant => $name,)+
                })
            }
        }
    };
}
