//! Debug-build shadow. Each thread keeps a table of its live regions and
//! their current `top`. Handles assert on every access that their block lies
//! below `top`; a failure means memory was reclaimed while still reachable,
//! which is a soundness bug in this crate. Release builds compile all of
//! this to nothing.

#[cfg(debug_assertions)]
mod imp {
    use std::cell::RefCell;

    struct Entry {
        base: usize,
        cap: usize,
        top: usize,
    }

    thread_local! {
        static REGIONS: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
    }

    pub(crate) fn register(base: usize, cap: usize) {
        if cap == 0 {
            return;
        }
        let _ = REGIONS.try_with(|r| r.borrow_mut().push(Entry { base, cap, top: 0 }));
    }

    pub(crate) fn unregister(base: usize) {
        let _ = REGIONS.try_with(|r| r.borrow_mut().retain(|e| e.base != base));
    }

    pub(crate) fn set_top(base: usize, top: usize) {
        let _ = REGIONS.try_with(|r| {
            if let Some(e) = r.borrow_mut().iter_mut().find(|e| e.base == base) {
                e.top = top;
            }
        });
    }

    pub(crate) fn check_live(addr: *const u8, bytes: usize) {
        if bytes == 0 {
            return;
        }
        let addr = addr as usize;
        let _ = REGIONS.try_with(|r| {
            let r = r.borrow();
            let e = r
                .iter()
                .find(|e| addr >= e.base && addr < e.base + e.cap)
                .expect("ordstack: internal error: block outside every live stack");
            assert!(
                addr - e.base + bytes <= e.top,
                "ordstack: internal error: block used after its memory was reclaimed"
            );
        });
    }
}

#[cfg(not(debug_assertions))]
mod imp {
    #[inline(always)]
    pub(crate) fn register(_base: usize, _cap: usize) {}
    #[inline(always)]
    pub(crate) fn unregister(_base: usize) {}
    #[inline(always)]
    pub(crate) fn set_top(_base: usize, _top: usize) {}
    #[inline(always)]
    pub(crate) fn check_live(_addr: *const u8, _bytes: usize) {}
}

pub(crate) use imp::*;

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn live_block_passes() {
        register(0x10_0000, 4096);
        set_top(0x10_0000, 64);
        check_live(core::ptr::without_provenance(0x10_0000), 64);
        check_live(core::ptr::without_provenance(0x10_0020), 0);
        unregister(0x10_0000);
    }

    #[test]
    #[should_panic(expected = "reclaimed")]
    fn block_above_top_panics() {
        register(0x20_0000, 4096);
        set_top(0x20_0000, 16);
        check_live(core::ptr::without_provenance(0x20_0008), 16);
    }
}
