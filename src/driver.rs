/// This mod is the `ats` driver.
/// 


#[cfg(feature = "no-simul")]
const ATS_BASE_ADDR: usize = 0xf000000;

const ATS_REGSPACE_LEN: usize = 0x1000;


#[cfg(any(feature = "locked-simul", feature = "lock-free-simul"))]
mod simul {
    use spin::{Lazy, Mutex};
    use crate::{executor::Executor, handler::Handler};
    extern "C" {
        fn cpu_num() -> usize;
    }
    pub static EXECUTOR: Lazy<Mutex<Executor>> = Lazy::new(|| Mutex::new(Executor::new()));
    pub static HANDLER: Lazy<Mutex<Handler>> = Lazy::new(|| Mutex::new(Handler::new(unsafe { cpu_num() })));
}


use ats_intc_pac::*;

use super::{TaskRef, Task};

#[derive(Debug, Clone, Copy)]
pub struct AtsDriver(usize);

impl AtsDriver {
    #[cfg(any(feature = "locked-simul", feature = "lock-free-simul", feature = "no-simul"))]
    pub fn new(base_addr: usize) -> Self {
        Self(base_addr)
    }

    // #[cfg(feature = "no-simul")]
    // pub fn new() -> Self {
    //     Self(ATS_BASE_ADDR)
    // }

    /// the mmio registers 
    fn regs(&self) -> &'static atsintc::RegisterBlock {
        unsafe { &*(self.0 as *const _) }
    }

    // New design of the hardware do not include reset field.

    // /// When software reset the hardware, it will set the `reset` bit in the `control` register.
    // /// It will busy wait the `reset` bit until it's clear.
    // pub fn reset(&self) {
    //     self.regs().control().write(|w| w.reset().set_bit());
    //     #[cfg(any(feature = "locked-simul", feature = "lock-free-simul"))]
    //     {
    //         // init the EXECUTOR & HANDLER
    //         use spin::Mutex;
    //         use crate::executor::Executor;
    //         use crate::handler::Handler;
    //         extern "C" {
    //             fn cpu_num() -> usize;
    //         }
    //         unsafe { *simul::EXECUTOR.as_mut_ptr() = Mutex::new(Executor::new()) };
    //         unsafe { *simul::HANDLER.as_mut_ptr() = Mutex::new(Handler::new(cpu_num())) };
    //         // clear the `reset` bit in the `control` register
    //         self.regs().control().write(|w| w.reset().clear_bit());
    //     }
    //     while !self.regs().control().read().reset().is_complete() {}
    // }

    /// When software load a handler task in the hardware, It will fill the `handler` and `irq` registers.
    /// Then it will set the `lhandler` bit in the `control` register.
    #[cfg(not(feature = "task-as-usize"))]
    pub fn load_handler(&self, irq: usize, handler: TaskRef) {
        #[cfg(any(feature = "locked-simul", feature = "lock-free-simul"))]
        {
            // The HANDLER load the specified task
            simul::HANDLER.lock().load(irq, handler);
        }

        #[cfg(feature = "no-simul")]
        {
            let raw_ptr = handler.as_ptr() as usize;
            self.regs().intc().enqueueregs(irq).write(|w| unsafe { w.bits(raw_ptr as u64) });
        }
    }

    #[cfg(feature = "task-as-usize")]
    pub fn load_handler(&self, irq: usize, handler: usize) {
        self.regs().intc().enqueueregs(irq).write(|w| unsafe { w.bits(handler as u64) });
    }

    /// Software store a task into the hardware executor.
    #[cfg(not(feature = "task-as-usize"))]
    pub fn stask(&self, task: TaskRef, process_id: usize, priority: usize) {
        #[cfg(any(feature = "locked-simul", feature = "lock-free-simul"))]
        {
            // The EXECUTOR spawn the specified task
            simul::EXECUTOR.lock().spawn(task, priority as _);
        }

        #[cfg(feature = "no-simul")]
        {
            let raw_ptr = task.as_ptr() as usize;
            self.regs().processes(process_id).ats().enqueueregs(priority).write(|w| unsafe { w.bits(raw_ptr as u64) });
        }
    }

    #[cfg(feature = "task-as-usize")]
    pub fn stask(&self, task: usize, process_id: usize, priority: usize) {
        self.regs().processes(process_id).ats().enqueueregs(priority).write(|w| unsafe { w.bits(task as u64) });
    }

    /// Software fetch a task from the hardware executor.
    #[cfg(not(feature = "task-as-usize"))]
    pub fn ftask(&self, process_id: usize) -> Option<TaskRef> {
        #[cfg(any(feature = "locked-simul", feature = "lock-free-simul"))]
        {
            // The EXECUTOR fetch the specified task
            if let Some(handler) = simul::HANDLER.lock().handle(0) {
                simul::EXECUTOR.lock().spawn(handler, 0);
            }
            simul::EXECUTOR.lock().fetch()
        }

        #[cfg(feature = "no-simul")]
        {
            let raw_ptr = self.regs().processes(process_id).ats().dequeue().read().bits() as usize;
            if raw_ptr == 0 {
                None
            } else {
                Some(unsafe { TaskRef::from_ptr(raw_ptr as *const Task) })
            }
        }
    }

    #[cfg(feature = "task-as-usize")]
    pub fn ftask(&self, process_id: usize) -> Option<usize> {
        let raw_ptr = self.regs().processes(process_id).ats().dequeue().read().bits() as usize;
        if raw_ptr == 0 {
            None
        } else {
            Some(raw_ptr)
        }
    }
}
