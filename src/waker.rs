//! This mod specific the waker related with coroutine
//!

use alloc::sync::Arc;

use crate::AtsIntc;

use super::task::TaskRef;
use core::{
    sync::atomic::Ordering,
    task::{RawWaker, RawWakerVTable, Waker},
};

const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

#[derive(Clone, Copy)]
struct WakerData {
    task_ref: TaskRef,
    hw: &'static AtsIntc,
}

impl WakerData {
    fn wake(&self) {
        let priority = unsafe { (*self.task_ref.as_ptr()).priority.load(Ordering::Acquire) };
        self.hw.ps_push(self.task_ref, priority as usize);
    }
}

unsafe fn clone(p: *const ()) -> RawWaker {
    let data = Arc::new(*(p as *const WakerData)).as_ref() as *const WakerData as *const ();
    RawWaker::new(data, &VTABLE)
}

unsafe fn wake(p: *const ()) {
    let waker = &*(p as *const WakerData);
    waker.wake();
}

unsafe fn drop(_p: *const ()) {}

///
pub unsafe fn new_waker(task_ref: TaskRef, hw: &'static AtsIntc) -> Waker {
    let data = Arc::new(WakerData { task_ref, hw }).as_ref() as *const WakerData as *const ();
    Waker::from_raw(RawWaker::new(data, &VTABLE))
}
