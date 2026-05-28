use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    edi: u32,
    esi: u32,
    ebx: u32,
    ebp: u32,
    eip: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Exited,
}

use crate::vfs::VfsNode;

pub struct Thread {
    id: usize,
    pub stack_ptr: u32,
    stack_allocated: Vec<u8>,
    state: ThreadState,
    pub file_descriptors: Vec<Option<(VfsNode, usize)>>,
}

impl Thread {
    pub fn new(id: usize) -> Self {
        // Pre-allocate FDs 0, 1, and 2 as reserved/empty for now
        let mut fds = Vec::new();
        fds.push(None); // 0: stdin
        fds.push(None); // 1: stdout
        fds.push(None); // 2: stderr

        Self {
            id,
            stack_ptr: 0,
            stack_allocated: Vec::new(),
            state: ThreadState::Ready,
            file_descriptors: fds,
        }
    }
}

pub struct Scheduler {
    ready_queue: VecDeque<Box<Thread>>,
    pub current_thread: Option<Box<Thread>>,
    next_id: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            current_thread: None,
            next_id: 1, // 0 is reserved for the main thread
        }
    }
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

#[unsafe(naked)]
pub unsafe extern "C" fn switch_to(old_esp: *mut u32, new_esp: u32) {
    unsafe {
        core::arch::naked_asm!(
            "push ebp",
            "push ebx",
            "push esi",
            "push edi",
            // old_esp is at esp + 20 (4 registers pushed * 4 bytes + 4 bytes for ret addr)
            // Wait, we pushed 4 registers, so old_esp was at esp + 4 before. Now it's at esp + 20.
            // new_esp was at esp + 8 before. Now it's at esp + 24.
            "mov eax, [esp + 20]", // load old_esp pointer into eax
            "mov [eax], esp",      // save current esp to *old_esp
            "mov esp, [esp + 24]", // load new_esp into esp
            "pop edi",
            "pop esi",
            "pop ebx",
            "pop ebp",
            "ret",
        );
    }
}

pub fn init() {
    let mut sched = SCHEDULER.lock();
    let mut main_thread = Box::new(Thread::new(0));
    main_thread.state = ThreadState::Running;
    sched.current_thread = Some(main_thread);
}

pub fn spawn(entry_point: fn()) {
    let mut sched = SCHEDULER.lock();
    let id = sched.next_id;
    sched.next_id += 1;

    let mut thread = Box::new(Thread::new(id));

    // Allocate 4KB stack
    let mut stack = Vec::<u8>::new();
    stack.resize(4096, 0);

    let stack_end = stack.as_ptr() as u32 + 4096;
    let aligned_stack_end = stack_end & !3; // Align to 4 bytes

    let context_ptr =
        (aligned_stack_end - core::mem::size_of::<TaskContext>() as u32) as *mut TaskContext;

    unsafe {
        *context_ptr = TaskContext {
            edi: 0,
            esi: 0,
            ebx: 0,
            ebp: 0,
            eip: entry_point as u32,
        };
    }

    thread.stack_ptr = context_ptr as u32;
    thread.stack_allocated = stack;

    sched.ready_queue.push_back(thread);
}

pub fn yield_task() {
    let mut sched = SCHEDULER.lock();

    if sched.ready_queue.is_empty() {
        return; // No other threads to run
    }

    // Take the current thread
    let mut current = sched.current_thread.take().expect("No current thread!");
    current.state = ThreadState::Ready;

    // Get the next thread
    let mut next = sched
        .ready_queue
        .pop_front()
        .expect("Ready queue is empty!");
    next.state = ThreadState::Running;

    let old_esp_ptr = &mut current.stack_ptr as *mut u32;
    let new_esp = next.stack_ptr;

    sched.ready_queue.push_back(current);
    sched.current_thread = Some(next);

    // Drop the lock before switching!
    drop(sched);

    unsafe {
        switch_to(old_esp_ptr, new_esp);
    }
}

#[unsafe(naked)]
pub unsafe extern "C" fn jump_to_usermode(user_eip: u32, user_esp: u32) -> ! {
    core::arch::naked_asm!(
        // 1. Fetch parameters into scratch registers before touching the stack.
        // In cdecl: [esp] is Return Address, [esp+4] is user_eip, [esp+8] is user_esp
        "mov eax, [esp + 4]", // eax = user_eip
        "mov ecx, [esp + 8]", // ecx = user_esp
        // 2. Clear out/set up data segments with the user data selector (RPL=3)
        "mov dx, 0x23",
        "mov ds, dx",
        "mov es, dx",
        "mov fs, dx",
        "mov gs, dx",
        // 3. Construct the IRET stack frame from BOTTOM to TOP
        "push 0x23",  // SS (User Data Selector, RPL=3)
        "push ecx",   // ESP (User Stack Pointer)
        "push 0x002", // EFLAGS (IF=0 to keep interrupts disabled for now)
        "push 0x1B",  // CS (User Code Selector, RPL=3)
        "push eax",   // EIP (User Entry Point)
        // 4. Execute the transition
        "iretd"
    );
}
