pub mod counter;
pub mod die;
pub mod fsm;
pub mod lookup;
pub mod pile;
pub mod player_map;
pub mod rotating;
mod strictness;
pub use strictness::Strictness;
pub mod verbatim;

pub(crate) type Rng = rand_chacha::ChaCha8Rng;

pub use spru_macro::FromInfallible;

pub trait Serial: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static {}

impl<T: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static> Serial for T {}

pub trait AddSigned: Sized + Send + Sync + Copy + std::fmt::Debug + std::fmt::Display {
    type Signed: Sized
        + Send
        + Sync
        + Copy
        + num_traits::Signed
        + std::fmt::Debug
        + std::fmt::Display;
    fn checked_add(self, rhs: Self::Signed) -> Option<Self>;
    fn saturating_add(self, rhs: Self::Signed) -> Self;
    fn into_signed(self) -> Self::Signed;
}

macro_rules! signed_unsigned {
    ($u:ty, $s:ty) => {
        signed_unsigned!($s, $s, checked_add, saturating_add);
        signed_unsigned!($u, $s, checked_add_signed, saturating_add_signed);
    };
    ($lhs:ty, $rhs:ty, $checked:ident, $saturating:ident) => {
        impl AddSigned for $lhs {
            type Signed = $rhs;
            fn checked_add(self, rhs: Self::Signed) -> Option<Self> {
                self.$checked(rhs)
            }

            fn saturating_add(self, rhs: Self::Signed) -> Self {
                self.$saturating(rhs)
            }

            fn into_signed(self) -> Self::Signed {
                self as $rhs
            }
        }
    };
}

signed_unsigned!(u8, i8);
signed_unsigned!(u16, i16);
signed_unsigned!(u32, i32);
signed_unsigned!(u64, i64);
signed_unsigned!(usize, isize);

#[cfg(test)]
mod test {
    #[test]
    fn add() {
        let a = 2u32;
        let b = -1i32;
        let c = super::AddSigned::checked_add(a, b);
        assert_eq!(c, Some(1));
        let a = 2u32;
        let b = -3i32;
        let c = super::AddSigned::checked_add(a, b);
        assert_eq!(c, None);
    }
}
