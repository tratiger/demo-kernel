use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use crate::fs::types::VfsNode;

#[derive(Debug, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Exited,
}

pub struct Thread {
    pub id: usize,
    pub stack_ptr: u32,
    pub stack_allocated: Vec<u8>,
    pub state: ThreadState,
    pub file_descriptors: Vec<Option<(VfsNode, usize)>>,
}

pub struct Scheduler {
    pub ready_queue: VecDeque<Box<Thread>>,
    pub current_thread: Option<Box<Thread>>,
    pub next_id: usize,
}

impl Thread {
    pub fn new(id: usize) -> Self {
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

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            current_thread: None,
            next_id: 1, // 0 is reserved for the main thread
        }
    }
}
