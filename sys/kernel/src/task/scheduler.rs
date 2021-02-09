// Scheduler

use crate::sync::atomicflags::AtomicBitflags;
use crate::window::winsys::*;
use crate::{arch::cpu::Cpu, system::System};
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::*;
use bitflags::*;
use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::num::NonZeroUsize;
use core::sync::atomic::*;
use core::time::Duration;

use crate::graphics::bitmap::*;
use crate::graphics::color::*;
use crate::graphics::coords::*;

static mut SCHEDULER: Option<Box<Scheduler>> = None;

static SCHEDULER_ENABLED: AtomicBool = AtomicBool::new(false);

pub struct Scheduler {
    urgent: ThreadQueue,
    ready: ThreadQueue,
    pool: ThreadPool,

    timer_events: Vec<TimerEvent>,
    next_timer: Timer,

    idle: ThreadHandle,
    current: ThreadHandle,
    retired: Option<ThreadHandle>,
}

impl Scheduler {
    /// Start scheduler and sleep forever
    pub(crate) unsafe fn start(f: fn(usize) -> (), args: usize) -> ! {
        const SIZE_OF_URGENT_QUEUE: usize = 100;
        const SIZE_OF_MAIN_QUEUE: usize = 250;

        let urgent = ThreadQueue::with_capacity(SIZE_OF_URGENT_QUEUE);
        let ready = ThreadQueue::with_capacity(SIZE_OF_MAIN_QUEUE);

        let mut pool = ThreadPool::default();
        let idle = {
            let idle = RawThread::new(ProcessId(0), Priority::Idle, "Idle", None, 0);
            let handle = idle.handle;
            pool.add(Box::new(idle));
            handle
        };

        SCHEDULER = Some(Box::new(Self {
            pool,
            ready,
            urgent,
            timer_events: Vec::with_capacity(100),
            next_timer: Timer::JUST,
            idle,
            current: idle,
            retired: None,
        }));

        SpawnOption::with_priority(Priority::Normal).spawn(f, args, "System");

        SCHEDULER_ENABLED.store(true, Ordering::SeqCst);

        loop {
            Cpu::halt();
        }
    }

    #[inline]
    #[track_caller]
    fn shared<'a>() -> &'a mut Self {
        unsafe { SCHEDULER.as_mut().unwrap() }
    }

    /// Get the current process if possible
    #[inline]
    pub fn current_pid() -> Option<ProcessId> {
        if Self::is_enabled() {
            Self::current_thread().map(|thread| thread.as_ref().pid)
        } else {
            None
        }
    }

    /// Get the current thread running on the current processor
    #[inline]
    pub fn current_thread() -> Option<ThreadHandle> {
        unsafe {
            Cpu::without_interrupts(|| {
                if Self::is_enabled() {
                    let shared = Self::shared();
                    Some(shared.current)
                } else {
                    None
                }
            })
        }
    }

    pub(crate) unsafe fn reschedule() {
        if Self::is_enabled() {
            Cpu::without_interrupts(|| {
                Self::process_timer_event();
                let shared = Self::shared();
                if shared.current.as_ref().priority != Priority::Realtime {
                    if shared.current.update(|current| current.quantum.consume()) {
                        Self::switch_context();
                    }
                }
            })
        }
    }

    pub fn sleep() {
        unsafe {
            Cpu::without_interrupts(|| {
                let shared = Self::shared();
                let current = shared.current;
                current.as_ref().attribute.insert(ThreadAttributes::ASLEEP);
                Self::switch_context();
            })
        }
    }

    pub fn yield_thread() {
        unsafe { Cpu::without_interrupts(|| Self::switch_context()) }
    }

    /// Get the next executable thread from the thread queue
    fn next() -> Option<ThreadHandle> {
        let shared = Self::shared();
        // if shared.is_frozen.load(Ordering::SeqCst) {
        //     return None;
        // }
        // if !sch.next_timer.until() {
        //     sch.sem_timer.signal();
        // }
        if let Some(next) = shared.urgent.dequeue() {
            return Some(next);
        }
        if let Some(next) = shared.ready.dequeue() {
            return Some(next);
        }
        None
    }

    fn retire(handle: ThreadHandle) {
        let shared = Self::shared();
        let thread = handle.as_ref();
        if thread.priority == Priority::Idle {
            return;
        } else if thread.attribute.contains(ThreadAttributes::ZOMBIE) {
            ThreadPool::drop_thread(handle);
        } else if thread.attribute.test_and_clear(ThreadAttributes::AWAKE) {
            thread.attribute.remove(ThreadAttributes::ASLEEP);
            shared.ready.enqueue(handle).unwrap();
        } else if thread.attribute.contains(ThreadAttributes::ASLEEP) {
            thread.attribute.remove(ThreadAttributes::QUEUED);
        } else {
            shared.ready.enqueue(handle).unwrap();
        }
    }

    /// Add thread to the queue
    fn add(handle: ThreadHandle) {
        let shared = Self::shared();
        let thread = handle.as_ref();
        if thread.priority == Priority::Idle || thread.attribute.contains(ThreadAttributes::ZOMBIE)
        {
            return;
        }
        if !thread.attribute.test_and_set(ThreadAttributes::QUEUED) {
            if thread.attribute.test_and_clear(ThreadAttributes::AWAKE) {
                thread.attribute.remove(ThreadAttributes::ASLEEP);
            }
            shared.ready.enqueue(handle).unwrap();
        }
    }

    pub fn schedule_timer(event: TimerEvent) -> Result<(), TimerEvent> {
        unsafe {
            Cpu::without_interrupts(|| {
                let shared = Self::shared();
                shared.timer_events.push(event);
                shared
                    .timer_events
                    .sort_by(|a, b| a.timer.deadline.cmp(&b.timer.deadline));
            });
            Self::process_timer_event();
            Ok(())
        }
    }

    unsafe fn process_timer_event() {
        Cpu::without_interrupts(|| {
            let shared = Self::shared();

            while let Some(event) = shared.timer_events.first() {
                if event.until() {
                    break;
                } else {
                    shared.timer_events.remove(0).fire();
                }
            }

            if let Some(event) = shared.timer_events.first() {
                shared.next_timer = event.timer;
            }
        })
    }

    /// Returns whether or not the thread scheduler is working.
    fn is_enabled() -> bool {
        unsafe { &SCHEDULER }.is_some() && SCHEDULER_ENABLED.load(Ordering::SeqCst)
    }

    #[track_caller]
    unsafe fn switch_context() {
        Cpu::assert_without_interrupt();

        let shared = Self::shared();
        let current = shared.current;
        let next = Self::next().unwrap_or(shared.idle);
        // current.update(|thread| {
        //     // TODO: update statistics
        // });
        if current != next {
            shared.retired = Some(current);
            shared.current = next;

            //-//-//-//-//
            Cpu::switch_context(
                &current.as_ref().context as *const _ as *mut _,
                &next.as_ref().context as *const _ as *mut _,
            );
            //-//-//-//-//

            let current = shared.current;
            current.update(|thread| {
                thread.attribute.remove(ThreadAttributes::AWAKE);
                thread.attribute.remove(ThreadAttributes::ASLEEP);
                // thread.measure.store(Timer::measure(), Ordering::SeqCst);
            });
            let retired = shared.retired.unwrap();
            shared.retired = None;
            Scheduler::retire(retired);
        }
    }

    fn spawn_f(
        start: ThreadStart,
        args: usize,
        name: &str,
        options: SpawnOption,
    ) -> Option<ThreadHandle> {
        let pid = if options.raise_pid {
            ProcessId::raise()
        } else {
            Self::current_pid().unwrap_or(ProcessId(0))
        };
        let thread = RawThread::new(pid, options.priority, name, Some(start), args);
        let thread = {
            let handle = thread.handle;
            ThreadPool::shared().add(Box::new(thread));
            handle
        };
        Self::add(thread);
        Some(thread)
    }
}

#[no_mangle]
pub unsafe extern "C" fn sch_setup_new_thread() {
    let shared = Scheduler::shared();
    // let current = shared.current;
    // current.update(|thread| {
    //     thread.measure.store(Timer::measure(), Ordering::SeqCst);
    // });
    if let Some(retired) = shared.retired {
        shared.retired = None;
        Scheduler::retire(retired);
    }
}

#[derive(Default)]
struct ThreadPool {
    data: BTreeMap<ThreadHandle, Arc<UnsafeCell<Box<RawThread>>>>,
}

impl ThreadPool {
    #[inline]
    #[track_caller]
    fn synchronized<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        unsafe { Cpu::without_interrupts(|| f()) }
    }

    #[inline]
    #[track_caller]
    fn shared<'a>() -> &'a mut Self {
        &mut Scheduler::shared().pool
    }

    fn add(&mut self, thread: Box<RawThread>) {
        Self::synchronized(|| {
            let handle = thread.handle;
            self.data.insert(handle, Arc::new(UnsafeCell::new(thread)));
        });
    }

    fn drop_thread(handle: ThreadHandle) {
        Self::synchronized(|| {
            let shared = Self::shared();
            shared.data.remove(&handle);
        });
    }

    fn get<'a>(&self, key: &ThreadHandle) -> Option<&'a Box<RawThread>> {
        Self::synchronized(|| self.data.get(key).map(|v| v.clone().get()))
            .map(|thread| unsafe { &(*thread) })
    }

    fn get_mut<F, R>(&mut self, key: &ThreadHandle, f: F) -> Option<R>
    where
        F: FnOnce(&mut RawThread) -> R,
    {
        let thread = Self::synchronized(move || self.data.get_mut(key).map(|v| v.clone()));
        thread.map(|thread| unsafe {
            let thread = thread.get();
            f(&mut *thread)
        })
    }
}

pub struct SpawnOption {
    pub priority: Priority,
    pub raise_pid: bool,
}

impl SpawnOption {
    #[inline]
    pub const fn new() -> Self {
        Self {
            priority: Priority::Normal,
            raise_pid: false,
        }
    }

    #[inline]
    pub const fn with_priority(priority: Priority) -> Self {
        Self {
            priority,
            raise_pid: false,
        }
    }

    // #[inline]
    // pub fn personality(mut self, personality: Box<dyn Personality>) -> Self {
    //     self.personality = Some(personality);
    //     self
    // }

    #[inline]
    pub fn spawn_f(self, start: fn(usize), args: usize, name: &str) -> Option<ThreadHandle> {
        Scheduler::spawn_f(start, args, name, self)
    }

    #[inline]
    pub fn spawn(mut self, start: fn(usize), args: usize, name: &str) -> Option<ThreadHandle> {
        self.raise_pid = true;
        Scheduler::spawn_f(start, args, name, self)
    }
}

static mut TIMER_SOURCE: Option<&'static dyn TimerSource> = None;

pub type TimeSpec = u64;

pub trait TimerSource {
    /// Create timer object from duration
    fn create(&self, duration: Duration) -> TimeSpec;

    /// Is that a timer before the deadline?
    fn until(&self, deadline: TimeSpec) -> bool;

    /// Get the value of the monotonic timer in microseconds
    fn monotonic(&self) -> Duration;
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Timer {
    deadline: TimeSpec,
}

impl Timer {
    pub const JUST: Timer = Timer { deadline: 0 };

    #[inline]
    pub fn new(duration: Duration) -> Self {
        let timer = unsafe { TIMER_SOURCE.as_ref().unwrap() };
        Timer {
            deadline: timer.create(duration),
        }
    }

    #[inline]
    pub const fn is_just(&self) -> bool {
        self.deadline == 0
    }

    #[inline]
    pub fn until(&self) -> bool {
        if self.is_just() {
            false
        } else {
            let timer = unsafe { TIMER_SOURCE.as_ref().unwrap() };
            timer.until(self.deadline)
        }
    }

    #[inline]
    pub(crate) unsafe fn set_timer(source: &'static dyn TimerSource) {
        TIMER_SOURCE = Some(source);
    }

    #[track_caller]
    pub fn sleep(duration: Duration) {
        if Scheduler::is_enabled() {
            let timer = Timer::new(duration);
            let mut event = TimerEvent::one_shot(timer);
            while timer.until() {
                match Scheduler::schedule_timer(event) {
                    Ok(()) => {
                        Scheduler::sleep();
                        return;
                    }
                    Err(e) => {
                        event = e;
                        Scheduler::yield_thread();
                    }
                }
            }
        } else {
            panic!("Scheduler unavailable");
        }
    }

    #[inline]
    pub fn usleep(us: u64) {
        Self::sleep(Duration::from_micros(us));
    }

    #[inline]
    pub fn msleep(ms: u64) {
        Self::sleep(Duration::from_millis(ms));
    }

    #[inline]
    pub fn monotonic() -> Duration {
        unsafe { TIMER_SOURCE.as_ref() }.unwrap().monotonic()
    }

    #[inline]
    pub fn measure() -> u64 {
        Self::monotonic().as_micros() as u64
    }
}

pub struct TimerEvent {
    timer: Timer,
    timer_type: TimerType,
}

#[derive(Debug, Copy, Clone)]
pub enum TimerType {
    OneShot(ThreadHandle),
    Window(WindowHandle, usize),
}

#[allow(dead_code)]
impl TimerEvent {
    pub fn one_shot(timer: Timer) -> Self {
        Self {
            timer,
            timer_type: TimerType::OneShot(Scheduler::current_thread().unwrap()),
        }
    }

    pub fn window(window: WindowHandle, timer_id: usize, timer: Timer) -> Self {
        Self {
            timer,
            timer_type: TimerType::Window(window, timer_id),
        }
    }

    pub fn until(&self) -> bool {
        self.timer.until()
    }

    pub fn fire(self) {
        match self.timer_type {
            TimerType::OneShot(thread) => thread.wake(),
            TimerType::Window(window, timer_id) => {
                todo!()
                // window.post(WindowMessage::Timer(timer_id)).unwrap()
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ProcessId(usize);

impl ProcessId {
    #[inline]
    fn raise() -> Self {
        static mut NEXT_ID: usize = 1;
        Self(unsafe { Cpu::interlocked_increment(&mut NEXT_ID) })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ThreadHandle(NonZeroUsize);

impl ThreadHandle {
    #[inline]
    fn new(val: usize) -> Option<Self> {
        NonZeroUsize::new(val).map(|x| Self(x))
    }

    /// Acquire the next thread ID
    #[inline]
    fn next() -> Self {
        static mut NEXT_ID: usize = 1;
        Self::new(unsafe { Cpu::interlocked_increment(&mut NEXT_ID) }).unwrap()
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0.get()
    }

    #[inline]
    #[track_caller]
    fn update<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut RawThread) -> R,
    {
        let shared = ThreadPool::shared();
        shared.get_mut(self, f).unwrap()
    }

    #[inline]
    fn get<'a>(&self) -> Option<&'a Box<RawThread>> {
        let shared = ThreadPool::shared();
        shared.get(self)
    }

    #[inline]
    #[track_caller]
    fn as_ref<'a>(&self) -> &'a RawThread {
        self.get().unwrap()
    }

    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.get().and_then(|v| v.name())
    }

    #[inline]
    fn wake(&self) {
        self.as_ref().attribute.insert(ThreadAttributes::AWAKE);
        Scheduler::add(*self);
    }

    // #[inline]
    // pub fn join(&self) -> usize {
    //     self.get().map(|t| t.sem.wait());
    //     0
    // }
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Ord, PartialOrd, Eq)]
pub enum Priority {
    Idle = 0,
    Low,
    Normal,
    High,
    Realtime,
}

impl Priority {
    pub fn is_useful(self) -> bool {
        match self {
            Priority::Idle => false,
            _ => true,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Quantum {
    current: u8,
    default: u8,
}

impl Quantum {
    const fn new(value: u8) -> Self {
        Quantum {
            current: value,
            default: value,
        }
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        self.current = self.default;
    }

    fn consume(&mut self) -> bool {
        if self.current > 1 {
            self.current -= 1;
            false
        } else {
            self.current = self.default;
            true
        }
    }
}

impl From<Priority> for Quantum {
    fn from(priority: Priority) -> Self {
        match priority {
            Priority::High => Quantum::new(25),
            Priority::Normal => Quantum::new(10),
            Priority::Low => Quantum::new(5),
            _ => Quantum::new(1),
        }
    }
}

const SIZE_OF_CONTEXT: usize = 512;
const SIZE_OF_STACK: usize = 0x10000;
const THREAD_NAME_LENGTH: usize = 32;

type ThreadStart = fn(usize) -> ();

#[allow(dead_code)]
struct RawThread {
    /// Architectural context data
    context: [u8; SIZE_OF_CONTEXT],
    stack: Option<Box<[u8]>>,

    // IDs
    pid: ProcessId,
    handle: ThreadHandle,

    // Properties
    // sem: Semaphore,
    // personality: Option<Box<dyn Personality>>,
    attribute: AtomicBitflags<ThreadAttributes>,
    priority: Priority,
    quantum: Quantum,

    // Statistics
    // measure: AtomicU64,
    // cpu_time: AtomicU64,
    // load0: AtomicU32,
    // load: AtomicU32,

    // Executor
    // executor: Option<Executor>,

    // Thread Name
    name: [u8; THREAD_NAME_LENGTH],
}

bitflags! {
    struct ThreadAttributes: usize {
        const QUEUED    = 0b0000_0000_0000_0001;
        const ASLEEP    = 0b0000_0000_0000_0010;
        const AWAKE     = 0b0000_0000_0000_0100;
        const ZOMBIE    = 0b0000_0000_0000_1000;
    }
}

impl Into<usize> for ThreadAttributes {
    fn into(self) -> usize {
        self.bits()
    }
}

impl AtomicBitflags<ThreadAttributes> {
    fn to_char(&self) -> char {
        if self.contains(ThreadAttributes::ZOMBIE) {
            'Z'
        } else if self.contains(ThreadAttributes::AWAKE) {
            'W'
        } else if self.contains(ThreadAttributes::ASLEEP) {
            'S'
        } else if self.contains(ThreadAttributes::QUEUED) {
            'R'
        } else {
            '-'
        }
    }
}

use core::fmt;
impl fmt::Display for AtomicBitflags<ThreadAttributes> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

impl RawThread {
    fn new(
        pid: ProcessId,
        priority: Priority,
        name: &str,
        start: Option<ThreadStart>,
        arg: usize,
    ) -> Self {
        let handle = ThreadHandle::next();

        let mut name_array = [0; THREAD_NAME_LENGTH];
        Self::set_name_array(&mut name_array, name);

        let mut thread = Self {
            context: [0; SIZE_OF_CONTEXT],
            stack: None,
            pid,
            handle,
            attribute: AtomicBitflags::empty(),
            priority,
            quantum: Quantum::from(priority),
            // measure: AtomicU64::new(0),
            // cpu_time: AtomicU64::new(0),
            // load0: AtomicU32::new(0),
            // load: AtomicU32::new(0),
            name: name_array,
        };
        if let Some(start) = start {
            unsafe {
                let mut stack = Vec::with_capacity(SIZE_OF_STACK);
                stack.resize(SIZE_OF_STACK, 0);
                let stack = stack.into_boxed_slice();
                thread.stack = Some(stack);
                let stack = thread.stack.as_mut().unwrap().as_mut_ptr() as *mut c_void;
                Cpu::make_new_thread(
                    thread.context.as_mut_ptr(),
                    stack.add(SIZE_OF_STACK),
                    start as usize,
                    arg,
                );
            }
        }
        thread
    }

    fn exit(&mut self) -> ! {
        // self.sem.signal();
        // self.personality.as_mut().map(|v| v.on_exit());
        // self.personality = None;

        // TODO:
        Timer::sleep(Duration::from_secs(2));
        self.attribute.insert(ThreadAttributes::ZOMBIE);
        // MyScheduler::sleep();
        unreachable!();
    }

    fn set_name_array(array: &mut [u8; THREAD_NAME_LENGTH], name: &str) {
        let mut i = 1;
        for c in name.bytes() {
            if i >= THREAD_NAME_LENGTH {
                break;
            }
            array[i] = c;
            i += 1;
        }
        array[0] = i as u8 - 1;
    }

    // fn set_name(&mut self, name: &str) {
    //     RawThread::set_name_array(&mut self.name, name);
    // }

    fn name<'a>(&self) -> Option<&'a str> {
        let len = self.name[0] as usize;
        match len {
            0 => None,
            _ => core::str::from_utf8(unsafe { core::slice::from_raw_parts(&self.name[1], len) })
                .ok(),
        }
    }
}

struct ThreadQueue {
    vec: Vec<NonZeroUsize>,
}

impl ThreadQueue {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity),
        }
    }

    fn dequeue(&mut self) -> Option<ThreadHandle> {
        unsafe {
            Cpu::without_interrupts(|| {
                if self.vec.len() > 0 {
                    Some(ThreadHandle(self.vec.remove(0)))
                } else {
                    None
                }
            })
        }
    }

    fn enqueue(&mut self, data: ThreadHandle) -> Result<(), ()> {
        unsafe {
            Cpu::without_interrupts(|| {
                self.vec.push(data.0);
                Ok(())
            })
        }
    }
}