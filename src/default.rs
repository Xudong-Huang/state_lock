/// we use special technique to avoid unstable specialization feature

pub trait NoDefaultImplement: Sized {
    type Ty;
    const HAS_DEFAULT: bool = false;
    fn tear_up() -> Self::Ty {
        unimplemented!("Default is not implemented for this type")
    }
}

impl<T> NoDefaultImplement for HasDefault<T> {
    type Ty = T;
}

pub struct HasDefault<T>(std::marker::PhantomData<T>);

impl<T: Default> HasDefault<T> {
    pub const HAS_DEFAULT: bool = true;
    pub fn tear_up() -> T {
        T::default()
    }
}

#[cfg(test)]
mod test {
    use super::{HasDefault, NoDefaultImplement};
    struct A;
    #[test]
    fn default_check() {
        let a = HasDefault::<i32>::HAS_DEFAULT;
        assert!(a);
        let b = HasDefault::<A>::HAS_DEFAULT;
        assert!(!b);

        assert_eq!(0, HasDefault::<i32>::tear_up());
    }

    #[test]
    #[should_panic]
    fn default_check_fail() {
        let _: A = HasDefault::<A>::tear_up();
    }
}
