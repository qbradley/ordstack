//! Trait impls that delegate to a handle's `Deref` target.

/// `deref_traits!([generics] Type, Target)` implements `Debug`, `PartialEq`,
/// `Eq`, `PartialOrd`, `Ord`, `Hash` and `Borrow<Target>` by delegation.
macro_rules! deref_traits {
    ([$($g:tt)*] $ty:ty, $target:ty) => {
        impl<$($g)*> core::fmt::Debug for $ty where $target: core::fmt::Debug {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Debug::fmt(&**self, f)
            }
        }
        impl<$($g)*> PartialEq for $ty where $target: PartialEq {
            fn eq(&self, other: &Self) -> bool {
                **self == **other
            }
        }
        impl<$($g)*> Eq for $ty where $target: Eq {}
        impl<$($g)*> PartialOrd for $ty where $target: PartialOrd {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                PartialOrd::partial_cmp(&**self, &**other)
            }
        }
        impl<$($g)*> Ord for $ty where $target: Ord {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                Ord::cmp(&**self, &**other)
            }
        }
        impl<$($g)*> core::hash::Hash for $ty where $target: core::hash::Hash {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                core::hash::Hash::hash(&**self, state)
            }
        }
        impl<$($g)*> core::borrow::Borrow<$target> for $ty {
            fn borrow(&self) -> &$target {
                self
            }
        }
    };
}
pub(crate) use deref_traits;
