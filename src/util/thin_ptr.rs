use std::sync::{Arc, Weak};

pub trait ThinPtr {
    fn thin_ptr(&self) -> *const ();
}

/// Arc::ptr_eq() are broken. See https://github.com/rust-lang/rust/issues/46139. Use this instead
impl<T: ?Sized> ThinPtr for Arc<T> {
    fn thin_ptr(&self) -> *const () {
        Arc::as_ptr(self) as *const ()
    }
}

/// Weak::ptr_eq() are broken. See https://github.com/rust-lang/rust/issues/46139. Use this instead
impl<T: ?Sized> ThinPtr for Weak<T> {
    fn thin_ptr(&self) -> *const () {
        match self.upgrade() {
            Some(arc) => arc.thin_ptr(),
            None => std::ptr::null(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::redundant_clone)]
    fn returns_same_for_weak_clones() {
        let arc = Arc::new(7);
        let a = Arc::downgrade(&arc.clone());
        let b = Arc::downgrade(&arc);
        assert_eq!(a.thin_ptr(), b.thin_ptr());
    }

    #[test]
    fn returns_same_for_arc_clones() {
        let a = Arc::new(7);
        let b = a.clone();
        assert_eq!(a.thin_ptr(), b.thin_ptr());
    }

    #[test]
    fn returns_same_for_arc_and_weak() {
        let arc = Arc::new(7);
        let weak = Arc::downgrade(&arc);
        assert_eq!(arc.thin_ptr(), weak.thin_ptr());
    }

    #[test]
    fn doesnt_return_null_for_arc() {
        let arc = Arc::new(7);
        assert_ne!(arc.thin_ptr(), std::ptr::null());
    }

    #[test]
    fn doesnt_return_null_for_weak() {
        let sink = Arc::new(7);
        let weak = Arc::downgrade(&sink);
        assert_ne!(weak.thin_ptr(), std::ptr::null());
    }

    #[test]
    fn returns_different_for_different_objects() {
        let a = Arc::new(7);
        let b = Arc::new(7);
        assert_ne!(a.thin_ptr(), b.thin_ptr());
    }

    #[test]
    fn returns_null_for_empty_weak() {
        let weak;
        {
            let arc = Arc::new(7);
            weak = Arc::downgrade(&arc);
        }
        assert!(weak.upgrade().is_none());
        assert_eq!(weak.thin_ptr(), std::ptr::null());
    }
}
