use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub struct KernelMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for KernelMutex<T> {}
unsafe impl<T: Send> Send for KernelMutex<T> {}

impl<T> KernelMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> KernelMutexGuard<'_, T> {
        let mut flags: u32;
        unsafe {
            core::arch::asm!(
                "pushfd",
                "pop {}",
                "cli",
                out(reg) flags,
                options(nomem, preserves_flags)
            );
        }

        while self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }

        KernelMutexGuard {
            mutex: self,
            flags,
        }
    }
}

pub struct KernelMutexGuard<'a, T> {
    mutex: &'a KernelMutex<T>,
    flags: u32,
}

impl<'a, T> Deref for KernelMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for KernelMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for KernelMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
        let flags = self.flags;
        unsafe {
            core::arch::asm!(
                "push {}",
                "popfd",
                in(reg) flags,
                options(nomem, preserves_flags)
            );
        }
    }
}
