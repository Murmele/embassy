use crate::control::{self, InResponse, OutResponse, Recipient, Request, RequestType};
use crate::driver::{Driver, Endpoint, EndpointError, EndpointIn, EndpointOut};
use crate::types::InterfaceNumber;
use crate::{Builder, Handler};
use core::cell::{Cell, RefCell};
use core::mem::{self, MaybeUninit};
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::waitqueue::WakerRegistration;

/// Sources:
/// - [1] Universal Serial Bus Mass Storage Class, Rev1.4: https://www.usb.org/sites/default/files/Mass_Storage_Specification_Overview_v1.4_2-19-2010.pdf
/// - [2] Universal Serial Bus Mass Storage Class Bulk-Only Transport, Rev1.0: https://www.usb.org/sites/default/files/usbmassbulk_10.pdf
/// - [3] USB Interface Association Descriptor Device Class Code and Use Model, Rev1.0: https://www.usb.org/sites/default/files/iadclasscode_r10.pdf

/// This should be used as `interface_class` when building the `UsbDevice`.
pub const USB_INTERFACE_CLASS: u8 = 0x08; // Mass storage class
pub const USB_SUBCLASS_SCSI: u8 = 0x06; // [1]
pub const USB_INTERFACE_SUBCLASS: u8 = USB_SUBCLASS_SCSI;
pub const USB_PROTOCOL_BULK_ONLY_TRANSPORT: u8 = 0x50;

// TODO: implement Reset and get max LUN [2]

/// Internal state
pub struct State<'a> {
    control: MaybeUninit<Control<'a>>,
    shared: ControlShared,
}

struct Control<'a> {
    ctrl_if_number: InterfaceNumber,
    shared: &'a ControlShared,
}

/// Shared data between Control and MassStorageSCSIClass
struct ControlShared {
    //line_coding: CriticalSectionMutex<Cell<LineCoding>>,
    //dtr: AtomicBool,
    //rts: AtomicBool,
    waker: RefCell<WakerRegistration>,
    changed: AtomicBool,
}

/// Implementation of
pub struct MassStorageSCSIClass<'d, D: Driver<'d>> {
    _com_ep: D::EndpointIn,
    // _data_if: InterfaceNumber,
    read_ep: D::EndpointOut,
    write_ep: D::EndpointIn,
    //control: &'d ControlShared,
}

impl<'d, D: Driver<'d>> MassStorageSCSIClass<'d, D> {
    pub fn new(builder: &mut Builder<'d, D>, state: &'d mut State<'d>, max_packet_size: u16) -> Self {
        // assert!(builder.control_buf_len() >= 7); // ???

        // Check out [3] for a better understanding

        let mut func = builder.function(
            USB_INTERFACE_CLASS,
            USB_INTERFACE_SUBCLASS,
            USB_PROTOCOL_BULK_ONLY_TRANSPORT,
        );

        // Interface 0
        let mut iface = func.interface();
        let ctrl_if_number = iface.interface_number();

        // Create interface descriptor
        let mut alt = iface.alt_setting(
            USB_INTERFACE_CLASS,
            USB_INTERFACE_SUBCLASS,
            USB_PROTOCOL_BULK_ONLY_TRANSPORT,
            None,
        );

        let comm_ep = alt.endpoint_interrupt_in(8, 255);

        let read_ep = alt.endpoint_bulk_out(max_packet_size);
        let write_ep = alt.endpoint_bulk_in(max_packet_size);

        drop(func);

        let control = state.control.write(Control {
            ctrl_if_number,
            shared: &state.shared,
        });
        builder.handler(control);

        MassStorageSCSIClass {
            _com_ep: comm_ep,
            read_ep,
            write_ep,
        }
    }
}

impl<'a> Default for State<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> State<'a> {
    /// Create a new `State`.
    pub fn new() -> Self {
        Self {
            control: MaybeUninit::uninit(),
            shared: ControlShared::default(),
        }
    }
}

impl Default for ControlShared {
    fn default() -> Self {
        ControlShared {
            // dtr: AtomicBool::new(false),
            // rts: AtomicBool::new(false),
            // line_coding: CriticalSectionMutex::new(Cell::new(LineCoding {
            //     stop_bits: StopBits::One,
            //     data_bits: 8,
            //     parity_type: ParityType::None,
            //     data_rate: 8_000,
            // })),
            waker: RefCell::new(WakerRegistration::new()),
            changed: AtomicBool::new(false),
        }
    }
}

impl<'d> Handler for Control<'d> {
    fn reset(&mut self) {
        // let shared = self.shared();
        // shared.line_coding.lock(|x| x.set(LineCoding::default()));
        // shared.dtr.store(false, Ordering::Relaxed);
        // shared.rts.store(false, Ordering::Relaxed);

        // shared.changed.store(true, Ordering::Relaxed);
        // shared.waker.borrow_mut().wake();
    }

    fn control_out(&mut self, req: control::Request, data: &[u8]) -> Option<OutResponse> {
        None
        // if (req.request_type, req.recipient, req.index)
        //     != (RequestType::Class, Recipient::Interface, self.ctrl_if_number.0 as u16)
        // {
        //     return None;
        // }

        // match req.request {
        //     REQ_SEND_ENCAPSULATED_COMMAND => {
        //         // We don't actually support encapsulated commands but pretend we do for standards
        //         // compatibility.
        //         Some(OutResponse::Accepted)
        //     }
        //     REQ_SET_LINE_CODING if data.len() >= 7 => {
        //         let coding = LineCoding {
        //             data_rate: u32::from_le_bytes(data[0..4].try_into().unwrap()),
        //             stop_bits: data[4].into(),
        //             parity_type: data[5].into(),
        //             data_bits: data[6],
        //         };
        //         let shared = self.shared();
        //         shared.line_coding.lock(|x| x.set(coding));
        //         debug!("Set line coding to: {:?}", coding);

        //         shared.changed.store(true, Ordering::Relaxed);
        //         shared.waker.borrow_mut().wake();

        //         Some(OutResponse::Accepted)
        //     }
        //     REQ_SET_CONTROL_LINE_STATE => {
        //         let dtr = (req.value & 0x0001) != 0;
        //         let rts = (req.value & 0x0002) != 0;

        //         let shared = self.shared();
        //         shared.dtr.store(dtr, Ordering::Relaxed);
        //         shared.rts.store(rts, Ordering::Relaxed);
        //         debug!("Set dtr {}, rts {}", dtr, rts);

        //         shared.changed.store(true, Ordering::Relaxed);
        //         shared.waker.borrow_mut().wake();

        //         Some(OutResponse::Accepted)
        //     }
        //     _ => Some(OutResponse::Rejected),
        // }
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        None
        // if (req.request_type, req.recipient, req.index)
        //     != (RequestType::Class, Recipient::Interface, self.ctrl_if_number.0 as u16)
        // {
        //     return None;
        // }

        // match req.request {
        //     // REQ_GET_ENCAPSULATED_COMMAND is not really supported - it will be rejected below.
        //     REQ_GET_LINE_CODING if req.length == 7 => {
        //         debug!("Sending line coding");
        //         let coding = self.shared().line_coding.lock(Cell::get);
        //         assert!(buf.len() >= 7);
        //         buf[0..4].copy_from_slice(&coding.data_rate.to_le_bytes());
        //         buf[4] = coding.stop_bits as u8;
        //         buf[5] = coding.parity_type as u8;
        //         buf[6] = coding.data_bits;
        //         Some(InResponse::Accepted(&buf[0..7]))
        //     }
        //     _ => Some(InResponse::Rejected),
        // }
    }
}
