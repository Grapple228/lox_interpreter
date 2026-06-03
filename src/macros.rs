#[macro_export]
macro_rules! define_enum_with_values {
    (
        $( #[$meta:meta] )*
        $vis:vis enum $name:ident {
            $(
                $( #[$variant_meta:meta] )*
                $variant:ident = $value:expr => $str:literal
            ),* $(,)?
        }
    ) => {
        $( #[$meta] )*
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis enum $name {
            $(
                $( #[$variant_meta] )*
                $variant = $value,
            )*
        }

        // From вместо Into (более идиоматично)
        impl From<$name> for u8 {
            fn from(op: $name) -> Self {
                op as u8
            }
        }

        impl $name {
            /// Convert to string slice
            #[inline]
            $vis const fn name(&self) -> &'static str {
                match self {
                    $( Self::$variant => $str, )*
                }
            }

            /// Convert from byte to enum
            #[inline]
            $vis const fn from_byte(byte: u8) -> Option<Self> {
                match byte {
                    $( $value => Some(Self::$variant), )*
                    _ => None,
                }
            }
        }
    };
}
