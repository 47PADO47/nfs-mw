//! Declarative macros that write the [`super::wire::Wire`] implementation of a struct or a field-less enum from its
//! definition, so the field order on the wire is the declaration order and cannot drift from it.

/// A struct whose fields are written in declaration order, with an optional `check(value) { ... }` block that refuses
/// a decoded value breaking a rule across fields or a range (it returns `Result<(), DecodeError>`).
macro_rules! wire_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $( $(#[$fmeta:meta])* pub $field:ident : $ty:ty ),* $(,)?
        }
        $( check($this:ident) $check:block )?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $( $(#[$fmeta])* pub $field: $ty, )*
        }

        impl $crate::message::wire::Wire for $name {
            fn put(&self, w: &mut $crate::message::codec::Writer<'_>) {
                let _ = &w;
                $( $crate::message::wire::Wire::put(&self.$field, w); )*
            }

            fn take(
                r: &mut $crate::message::codec::Reader<'_>,
            ) -> Result<Self, $crate::message::codec::DecodeError> {
                let _ = &r;
                let value = Self { $( $field: <$ty as $crate::message::wire::Wire>::take(r)?, )* };
                $(
                    {
                        let $this = &value;
                        let checked: Result<(), $crate::message::codec::DecodeError> = $check;
                        checked?;
                    }
                )?
                Ok(value)
            }
        }
    };
}

/// A field-less enum written as a varint of its `u8` code. An unknown code is [`super::DecodeError::BadEnum`].
/// Variants are append-only: a removed variant keeps its number reserved (protocol.md section 8.2).
macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $value:literal ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum $name {
            $( $(#[$vmeta])* $variant = $value, )*
        }

        impl $name {
            /// The wire code.
            pub fn code(self) -> u8 {
                self as u8
            }

            /// The variant with wire code `code`, or `None`.
            pub fn from_code(code: u8) -> Option<Self> {
                match code {
                    $( $value => Some(Self::$variant), )*
                    _ => None,
                }
            }
        }

        impl $crate::message::wire::Wire for $name {
            fn put(&self, w: &mut $crate::message::codec::Writer<'_>) {
                w.varint(u64::from(self.code()));
            }

            fn take(
                r: &mut $crate::message::codec::Reader<'_>,
            ) -> Result<Self, $crate::message::codec::DecodeError> {
                let code = r.varint(u64::from(u8::MAX)).map_err(|_| $crate::message::codec::DecodeError::BadEnum)?;
                let code = u8::try_from(code).map_err(|_| $crate::message::codec::DecodeError::BadEnum)?;
                Self::from_code(code).ok_or($crate::message::codec::DecodeError::BadEnum)
            }
        }
    };
}

pub(crate) use wire_enum;
pub(crate) use wire_struct;
