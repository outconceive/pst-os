// IRQ handler registration for Privion OS.
//
// Sequence:
//   1. Allocate a notification object (for the kernel to signal).
//   2. Call seL4_IRQControl_Get to get a handler cap for the IRQ number.
//   3. Bind the notification to the handler (seL4_IRQHandler_SetNotification).
//
// After registration, the driver calls wait() to block until the IRQ fires,
// then handles the interrupt, then calls ack() to re-enable it.

use sel4_sys::{
    seL4_CPtr, seL4_NoError, seL4_Error, seL4_Word,
    seL4_CapIRQControl, seL4_CapInitThreadCNode,
    seL4_IRQControl_Get, seL4_IRQHandler_SetNotification, seL4_IRQHandler_Ack,
};
use crate::mem::{UntypedAllocator, AllocError};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum IrqError {
    AllocFailed(AllocError),
    GetFailed(seL4_Error),
    BindFailed(seL4_Error),
}

impl From<AllocError> for IrqError {
    fn from(e: AllocError) -> Self { IrqError::AllocFailed(e) }
}

// ---------------------------------------------------------------------------
// IrqHandler
// ---------------------------------------------------------------------------

/// A registered IRQ — holds the handler cap and the bound notification cap.
pub struct IrqHandler {
    pub handler_cap: seL4_CPtr,
    pub notif_cap:   seL4_CPtr,
}

impl IrqHandler {
    /// Register an IRQ: allocate a notification, get the handler cap,
    /// and bind them together.
    ///
    /// Must be called from the initial thread (init), which holds
    /// seL4_CapIRQControl.
    pub fn register(irq: u8, alloc: &mut UntypedAllocator) -> Result<Self, IrqError> {
        // Allocate a notification for the kernel to signal on interrupt.
        let notif = alloc.create_notification()?;

        // Reserve a CSpace slot for the IRQ handler cap.
        // seL4_IRQControl_Get fills the slot itself.
        let handler = alloc.next_slot();
        let err = unsafe {
            seL4_IRQControl_Get(
                seL4_CapIRQControl,
                irq as seL4_Word,
                seL4_CapInitThreadCNode,
                handler,
                64, // depth of init's root CNode
            )
        };
        if err != seL4_NoError {
            return Err(IrqError::GetFailed(err));
        }

        // Bind the notification to the handler.
        let err = unsafe { seL4_IRQHandler_SetNotification(handler, notif) };
        if err != seL4_NoError {
            return Err(IrqError::BindFailed(err));
        }

        Ok(Self { handler_cap: handler, notif_cap: notif })
    }

    /// Block until this IRQ fires (waits on the notification).
    /// Uses native inline asm — safe for service processes without TLS.
    pub fn wait(&self) {
        unsafe { sel4_sys::native::sel4_wait_notification(self.notif_cap) };
    }

    /// Re-enable the IRQ at the hardware level.
    /// Must be called after each interrupt is handled.
    pub fn ack(&self) {
        unsafe { seL4_IRQHandler_Ack(self.handler_cap); }
    }
}
