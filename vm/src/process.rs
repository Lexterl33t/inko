use std::cell::UnsafeCell;
use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::{Arc, Mutex};

use binding::RcBinding;
use block::Block;
use compiled_code::CompiledCodePointer;
use config::Config;
use deref_pointer::DerefPointer;
use execution_context::ExecutionContext;
use global_scope::GlobalScopePointer;
use immix::block_list::BlockList;
use immix::copy_object::CopyObject;
use immix::global_allocator::RcGlobalAllocator;
use immix::local_allocator::LocalAllocator;
use mailbox::Mailbox;
use object_pointer::ObjectPointer;
use object_value;
use process_table::PID;
use vm::state::RcState;

pub type RcProcess = Arc<Process>;

#[derive(Debug)]
pub enum ProcessStatus {
    /// The process has been (re-)scheduled for execution.
    Scheduled,

    /// The process is running.
    Running,

    /// The process has been suspended.
    Suspended,

    /// The process has been suspended for garbage collection.
    SuspendForGc,

    /// The process is waiting for a message to arrive.
    WaitingForMessage,

    /// The process has finished execution.
    Finished,
}

impl ProcessStatus {
    pub fn is_running(&self) -> bool {
        match self {
            &ProcessStatus::Running => true,
            _ => false,
        }
    }
}

pub struct LocalData {
    /// The process-local memory allocator.
    pub allocator: LocalAllocator,

    /// The current execution context of this process.
    pub context: Box<ExecutionContext>,

    /// The mailbox for sending/receiving messages.
    ///
    /// The Mailbox is stored in LocalData as a Mailbox uses internal locking
    /// while still allowing a receiver to mutate it without a lock. This means
    /// some operations need a &mut self, which won't be possible if a Mailbox
    /// is stored directly in a Process.
    pub mailbox: Mailbox,

    /// The number of young garbage collections that have been performed.
    pub young_collections: usize,

    /// The number of mature garbage collections that have been performed.
    pub mature_collections: usize,

    /// The number of mailbox collections that have been performed.
    pub mailbox_collections: usize,

    /// The ID of the pool that this process belongs to.
    pub pool_id: usize,
}

pub struct Process {
    /// The process identifier of this process.
    pub pid: PID,

    /// The status of this process.
    pub status: Mutex<ProcessStatus>,

    /// Data stored in a process that should only be modified by a single thread
    /// at once.
    pub local_data: UnsafeCell<LocalData>,
}

unsafe impl Sync for LocalData {}
unsafe impl Send for LocalData {}
unsafe impl Sync for Process {}

impl Process {
    pub fn new(
        pid: PID,
        pool_id: usize,
        context: ExecutionContext,
        global_allocator: RcGlobalAllocator,
        config: &Config,
    ) -> RcProcess {
        let local_data = LocalData {
            allocator: LocalAllocator::new(global_allocator.clone(), config),
            context: Box::new(context),
            mailbox: Mailbox::new(global_allocator, config),
            young_collections: 0,
            mature_collections: 0,
            mailbox_collections: 0,
            pool_id: pool_id,
        };

        let process = Process {
            pid: pid,
            status: Mutex::new(ProcessStatus::Scheduled),
            local_data: UnsafeCell::new(local_data),
        };

        Arc::new(process)
    }

    pub fn from_block(
        pid: PID,
        pool_id: usize,
        block: &Block,
        global_allocator: RcGlobalAllocator,
        config: &Config,
    ) -> RcProcess {
        let context = ExecutionContext::from_isolated_block(block);

        Process::new(pid, pool_id, context, global_allocator, config)
    }

    pub fn set_pool_id(&self, id: usize) {
        self.local_data_mut().pool_id = id;
    }

    pub fn pool_id(&self) -> usize {
        self.local_data().pool_id
    }

    pub fn local_data_mut(&self) -> &mut LocalData {
        unsafe { &mut *self.local_data.get() }
    }

    pub fn local_data(&self) -> &LocalData {
        unsafe { &*self.local_data.get() }
    }

    pub fn push_context(&self, context: ExecutionContext) {
        let mut boxed = Box::new(context);
        let local_data = self.local_data_mut();
        let ref mut target = local_data.context;

        mem::swap(target, &mut boxed);

        target.set_parent(boxed);
    }

    pub fn status_integer(&self) -> usize {
        match *lock!(self.status) {
            ProcessStatus::Scheduled => 0,
            ProcessStatus::Running => 1,
            ProcessStatus::Suspended => 2,
            ProcessStatus::SuspendForGc => 3,
            ProcessStatus::WaitingForMessage => 4,
            ProcessStatus::Finished => 5,
        }
    }

    /// Pops an execution context.
    ///
    /// This method returns true if we're at the top of the execution context
    /// stack.
    pub fn pop_context(&self) -> bool {
        let local_data = self.local_data_mut();

        if let Some(parent) = local_data.context.parent.take() {
            local_data.context = parent;

            false
        } else {
            true
        }
    }

    pub fn get_register(&self, register: usize) -> ObjectPointer {
        self.local_data().context.get_register(register)
    }

    pub fn set_register(&self, register: usize, value: ObjectPointer) {
        self.local_data_mut().context.set_register(register, value);
    }

    pub fn set_local(&self, index: usize, value: ObjectPointer) {
        self.local_data_mut().context.set_local(index, value);
    }

    pub fn get_local(&self, index: usize) -> ObjectPointer {
        self.local_data().context.get_local(index)
    }

    pub fn local_exists(&self, index: usize) -> bool {
        let local_data = self.local_data();

        local_data.context.binding.local_exists(index)
    }

    pub fn set_global(&self, index: usize, value: ObjectPointer) {
        self.local_data_mut().context.set_global(index, value);
    }

    pub fn get_global(&self, index: usize) -> ObjectPointer {
        self.local_data().context.get_global(index)
    }

    pub fn allocate_empty(&self) -> ObjectPointer {
        self.local_data_mut().allocator.allocate_empty()
    }

    pub fn allocate(
        &self,
        value: object_value::ObjectValue,
        proto: ObjectPointer,
    ) -> ObjectPointer {
        let local_data = self.local_data_mut();

        local_data.allocator.allocate_with_prototype(value, proto)
    }

    pub fn allocate_without_prototype(
        &self,
        value: object_value::ObjectValue,
    ) -> ObjectPointer {
        let local_data = self.local_data_mut();

        local_data.allocator.allocate_without_prototype(value)
    }

    /// Sends a message to the current process.
    pub fn send_message(&self, sender: &RcProcess, message: ObjectPointer) {
        if sender.pid == self.pid {
            self.local_data_mut().mailbox.send_from_self(message);
        } else {
            self.local_data_mut().mailbox.send_from_external(message);
        }
    }

    /// Returns a message from the mailbox.
    pub fn receive_message(&self) -> Option<ObjectPointer> {
        let local_data = self.local_data_mut();
        let (should_copy, pointer_opt) = local_data.mailbox.receive();

        if let Some(mailbox_pointer) = pointer_opt {
            let pointer = if should_copy {
                // When another process sends us a message, the message will be
                // copied onto the mailbox heap. We can't directly use such a
                // pointer, as it might be garbage collected when it no longer
                // resides in the mailbox (e.g. after a receive).
                //
                // To work around this, we move the data from the mailbox heap
                // into the process' local heap.
                local_data.allocator.move_object(mailbox_pointer)
            } else {
                mailbox_pointer
            };

            Some(pointer)
        } else {
            None
        }
    }

    pub fn advance_instruction_index(&self) {
        self.local_data_mut().context.instruction_index += 1;
    }

    pub fn binding(&self) -> RcBinding {
        self.context().binding()
    }

    pub fn global_scope(&self) -> &GlobalScopePointer {
        &self.context().global_scope
    }

    pub fn context(&self) -> &Box<ExecutionContext> {
        &self.local_data().context
    }

    pub fn context_mut(&self) -> &mut Box<ExecutionContext> {
        &mut self.local_data_mut().context
    }

    pub fn compiled_code(&self) -> CompiledCodePointer {
        self.context().code.clone()
    }

    pub fn available_for_execution(&self) -> bool {
        match *lock!(self.status) {
            ProcessStatus::Scheduled => true,
            _ => false,
        }
    }

    pub fn set_status(&self, new_status: ProcessStatus) {
        let mut status = lock!(self.status);

        *status = new_status;
    }

    pub fn running(&self) {
        self.set_status(ProcessStatus::Running);
    }

    pub fn finished(&self) {
        self.set_status(ProcessStatus::Finished);
    }

    pub fn scheduled(&self) {
        self.set_status(ProcessStatus::Scheduled);
    }

    pub fn suspended(&self) {
        self.set_status(ProcessStatus::Suspended);
    }

    pub fn suspend_for_gc(&self) {
        self.set_status(ProcessStatus::SuspendForGc);
    }

    pub fn waiting_for_message(&self) {
        self.set_status(ProcessStatus::WaitingForMessage);
    }

    pub fn is_waiting_for_message(&self) -> bool {
        match *lock!(self.status) {
            ProcessStatus::WaitingForMessage => true,
            _ => false,
        }
    }

    pub fn wakeup_after_suspension_timeout(&self) {
        if self.is_waiting_for_message() {
            // When a timeout expires we don't want to retry the last
            // instruction as otherwise we'd end up in an infinite loop if
            // no message is received.
            self.advance_instruction_index();
        }
    }

    pub fn has_messages(&self) -> bool {
        self.local_data().mailbox.has_messages()
    }

    pub fn should_collect_young_generation(&self) -> bool {
        self.local_data().allocator.should_collect_young()
    }

    pub fn should_collect_mature_generation(&self) -> bool {
        self.local_data().allocator.should_collect_mature()
    }

    pub fn should_collect_mailbox(&self) -> bool {
        self.local_data().mailbox.allocator.should_collect()
    }

    pub fn contexts(&self) -> Vec<&ExecutionContext> {
        self.context().contexts().collect()
    }

    pub fn has_remembered_objects(&self) -> bool {
        self.local_data().allocator.has_remembered_objects()
    }

    /// Write barrier for tracking cross generation writes.
    ///
    /// This barrier is based on the Steele write barrier and tracks the object
    /// that is *written to*, not the object that is being written.
    pub fn write_barrier(
        &self,
        written_to: ObjectPointer,
        written: ObjectPointer,
    ) {
        if written_to.is_mature() && written.is_young() {
            self.local_data_mut().allocator.remember_object(written_to);
        }
    }

    pub fn prepare_for_collection(&self, mature: bool) -> bool {
        self.local_data_mut()
            .allocator
            .prepare_for_collection(mature)
    }

    pub fn reclaim_blocks(&self, state: &RcState, mature: bool) {
        self.local_data_mut()
            .allocator
            .reclaim_blocks(state, mature);
    }

    pub fn reclaim_all_blocks(&self) -> BlockList {
        let local_data = self.local_data_mut();
        let mut blocks = BlockList::new();

        for bucket in local_data.allocator.young_generation.iter_mut() {
            blocks.append(&mut bucket.blocks);
        }

        blocks.append(&mut local_data.allocator.mature_generation.blocks);
        blocks.append(&mut local_data.mailbox.allocator.bucket.blocks);

        blocks
    }

    pub fn reclaim_and_finalize(&self, state: &RcState) {
        let mut blocks = self.reclaim_all_blocks();

        for block in blocks.iter_mut() {
            block.reset_mark_bitmaps();
            block.prepare_finalization();
            block.reset();

            state.finalizer_pool.schedule(DerefPointer::new(block));
        }

        state.global_allocator.add_blocks(&mut blocks);
    }

    pub fn update_collection_statistics(&self, mature: bool) {
        let local_data = self.local_data_mut();

        local_data.allocator.update_collection_statistics();

        if mature {
            local_data.mature_collections += 1;
        } else {
            local_data.young_collections += 1;
        }
    }

    pub fn update_mailbox_collection_statistics(&self) {
        let local_data = self.local_data_mut();

        local_data.mailbox_collections += 1;
        local_data.mailbox.allocator.update_collection_statistics();
    }

    pub fn is_main(&self) -> bool {
        self.pid == 0
    }
}

impl PartialEq for Process {
    fn eq(&self, other: &Process) -> bool {
        self.pid == other.pid
    }
}

impl Eq for Process {}

impl Hash for Process {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pid.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use object_value;
    use vm::test::setup;

    #[test]
    fn test_contexts() {
        let (_machine, _block, process) = setup();

        assert_eq!(process.contexts().len(), 1);
    }

    #[test]
    fn test_update_collection_statistics_without_mature() {
        let (_machine, _block, process) = setup();

        process.update_collection_statistics(false);

        let local_data = process.local_data();

        assert_eq!(local_data.young_collections, 1);
    }

    #[test]
    fn test_update_collection_statistics_with_mature() {
        let (_machine, _block, process) = setup();

        process.update_collection_statistics(true);

        let local_data = process.local_data();

        assert_eq!(local_data.young_collections, 0);
        assert_eq!(local_data.mature_collections, 1);
    }

    #[test]
    fn test_update_mailbox_collection_statistics() {
        let (_machine, _block, process) = setup();

        process.update_mailbox_collection_statistics();

        let local_data = process.local_data();

        assert_eq!(local_data.mailbox_collections, 1);
    }

    #[test]
    fn test_receive_message() {
        let (machine, _block, process) = setup();

        // Simulate sending a message from an external process.
        let input_message = process
            .allocate(object_value::integer(14), process.allocate_empty());

        let attr = machine.state.intern(&"hello".to_string());

        input_message.add_attribute(&process, attr, attr);

        process
            .local_data_mut()
            .mailbox
            .send_from_external(input_message);

        let received = process.receive_message().unwrap();

        assert!(received.is_young());
        assert!(received.get().value.is_integer());
        assert!(received.get().prototype().is_some());
        assert!(received.get().attributes_map().is_some());
        assert!(received.is_finalizable());
    }
}
