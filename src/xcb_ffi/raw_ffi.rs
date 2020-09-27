use std::ptr::NonNull;

#[cfg(not(all(test, unix)))]
use libc::c_void;
#[cfg(unix)]
pub(crate) use libc::iovec;
use libc::{c_char, c_int, c_uint};

#[cfg(all(not(test), feature = "dl-libxcb"))]
use crate::errors::LibxcbLoadError;

// As defined in xcb_windefs.h
#[cfg(not(unix))]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub(crate) struct iovec {
    pub(crate) iov_base: *mut c_void,
    pub(crate) iov_len: c_int,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub(crate) struct xcb_connection_t {
    _unused: [u8; 0],
}

#[derive(Debug)]
pub(crate) struct XCBConnectionWrapper {
    ptr: NonNull<xcb_connection_t>,
    should_drop: bool,
}

// libxcb is fully thread-safe (well, except for xcb_disconnect()), so the following is
// actually fine and safe:
unsafe impl Send for XCBConnectionWrapper {}
unsafe impl Sync for XCBConnectionWrapper {}

impl Drop for XCBConnectionWrapper {
    fn drop(&mut self) {
        if self.should_drop {
            unsafe {
                xcb_disconnect(self.ptr.as_ptr());
            }
        }
    }
}

impl XCBConnectionWrapper {
    pub(crate) unsafe fn new(ptr: *mut xcb_connection_t, should_drop: bool) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            should_drop,
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut xcb_connection_t {
        self.ptr.as_ptr()
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub(crate) struct xcb_generic_event_t {
    pub(crate) response_type: u8,
    pub(crate) pad0: u8,
    pub(crate) sequence: u16,
    pub(crate) pad: [u32; 7],
    pub(crate) full_sequence: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub(crate) struct xcb_generic_error_t {
    pub(crate) response_type: u8,
    pub(crate) error_code: u8,
    pub(crate) sequence: u16,
    pub(crate) resource_id: u32,
    pub(crate) minor_code: u16,
    pub(crate) major_code: u8,
    pub(crate) pad0: u8,
    pub(crate) pad: [u32; 5],
    pub(crate) full_sequence: u32,
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
#[repr(C)]
pub(crate) struct xcb_void_cookie_t {
    pub(crate) sequence: c_uint,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub(crate) struct xcb_extension_t {
    pub(crate) name: *const c_char,
    pub(crate) global_id: c_int,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub(crate) struct xcb_protocol_request_t {
    pub(crate) count: usize,
    pub(crate) ext: *mut xcb_extension_t,
    pub(crate) opcode: u8,
    pub(crate) isvoid: u8,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub(crate) struct xcb_setup_t {
    _unused: [u8; 0],
}

pub(crate) mod connection_errors {
    use std::os::raw::c_int;

    pub(crate) const ERROR: c_int = 1;
    pub(crate) const EXT_NOTSUPPORTED: c_int = 2;
    pub(crate) const MEM_INSUFFICIENT: c_int = 3;
    pub(crate) const REQ_LEN_EXCEED: c_int = 4;
    pub(crate) const PARSE_ERR: c_int = 5;
    pub(crate) const INVALID_SCREEN: c_int = 6;
    pub(crate) const FDPASSING_FAILED: c_int = 7;
}

pub(crate) mod send_request_flags {
    use libc::c_int;

    pub(crate) const CHECKED: c_int = 1;
    pub(crate) const RAW: c_int = 2;
    //pub(crate) const DISCARD_REPLY: c_int = 4;
    pub(crate) const REPLY_FDS: c_int = 8;
}

#[cfg(all(not(test), feature = "dl-libxcb"))]
struct LibxcbLibrary {
    // Needed to keep the library loaded
    _library: libloading::Library,
    funcs: LibxcbFuncs,
}

#[cfg(all(not(test), feature = "dl-libxcb"))]
impl LibxcbLibrary {
    fn open_lib() -> Result<libloading::Library, LibxcbLoadError> {
        // TODO: Names for non-unix platforms
        #[cfg(unix)]
        const LIB_NAMES: &[&str] = &["libxcb.so.1", "libxcb.so"];
        #[cfg(not(unix))]
        compile_error!("dl-libxcb feature is not supported on non-unix");

        let mut errors = Vec::new();
        for lib_name in LIB_NAMES.iter() {
            match libloading::Library::new(lib_name) {
                Ok(library) => return Ok(library),
                Err(e) => errors.push((lib_name.into(), e.to_string())),
            }
        }
        Err(LibxcbLoadError::OpenLibError(errors))
    }

    /// # Safety
    ///
    /// The functions pointers in `funcs` do not have lifetime,
    /// but they must not outlive the returned result.
    #[cold]
    #[inline(never)]
    unsafe fn load() -> Result<Self, LibxcbLoadError> {
        let library = Self::open_lib()?;
        let funcs = LibxcbFuncs::new(&library)
            .map_err(|(symbol, e)| LibxcbLoadError::GetSymbolError(symbol.into(), e.to_string()))?;
        Ok(Self {
            _library: library,
            funcs,
        })
    }
}

#[cfg(all(not(test), feature = "dl-libxcb"))]
use once_cell::sync::Lazy;

#[cfg(all(not(test), feature = "dl-libxcb"))]
static LIBXCB_LIBRARY: Lazy<Result<LibxcbLibrary, LibxcbLoadError>> =
    Lazy::new(|| unsafe { LibxcbLibrary::load() });

#[cfg(all(not(test), feature = "dl-libxcb"))]
fn get_libxcb() -> &'static LibxcbLibrary {
    #[cold]
    #[inline(never)]
    fn failed(e: &LibxcbLoadError) -> ! {
        panic!("failed to load libxcb: {}", e);
    }
    match *LIBXCB_LIBRARY {
        Ok(ref library) => library,
        Err(ref e) => failed(e),
    }
}

/// Tries to dynamically load libxcb, returning an error on failure.
///
/// It is not required to call this function, as libxcb will be lazily loaded.
/// However, if a lazy load fails, a panic will be raised, missing the chance
/// to (nicely) handle the error.
///
/// It is safe to call this function more than once from the same or different
/// threads. Only the first call will try to load libxcb, subsequent calls will
/// always return the same result.
#[cfg(all(not(test), feature = "dl-libxcb"))]
pub fn load_libxcb() -> Result<(), LibxcbLoadError> {
    match Lazy::force(&LIBXCB_LIBRARY) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

#[cfg(not(test))]
macro_rules! make_ffi_fn_defs {
    {
        $(
            $(#[$fn_attr:meta])*
            fn $fn_name:ident($($fn_arg_name:ident: $fn_arg_type:ty),*) $(-> $fn_ret_ty:ty)?;
        )*
    } => {
        #[cfg(not(feature = "dl-libxcb"))]
        #[link(name = "xcb")]
        extern "C" {
            $(
                $(#[$fn_attr])*
                pub(crate) fn $fn_name($($fn_arg_name: $fn_arg_type),*) $(-> $fn_ret_ty)?;
            )*
        }

        #[cfg(feature = "dl-libxcb")]
        struct LibxcbFuncs {
            $(
                $(#[$fn_attr])*
                $fn_name: fn($($fn_arg_name: $fn_arg_type),*) $(-> $fn_ret_ty)?,
            )*
        }

        impl LibxcbFuncs {
            unsafe fn new(library: &libloading::Library) -> Result<Self, (&'static [u8], libloading::Error)> {
                Ok(Self {
                    $($fn_name: {
                        let symbol_name = concat!(stringify!($fn_name), "\0").as_bytes();
                        *library.get(symbol_name).map_err(|e| (symbol_name, e))?
                    },)*
                })
            }
        }

        $(
            #[cfg(feature = "dl-libxcb")]
            $(#[$fn_attr])*
            pub(crate) unsafe fn $fn_name($($fn_arg_name: $fn_arg_type),*) $(-> $fn_ret_ty)? {
                (get_libxcb().funcs.$fn_name)($($fn_arg_name),*)
            }
        )*
    };
}

#[cfg(not(test))]
make_ffi_fn_defs! {
    // From xcb.h
    fn xcb_flush(c: *mut xcb_connection_t) -> c_int;
    fn xcb_get_maximum_request_length(c: *mut xcb_connection_t) -> u32;
    fn xcb_prefetch_maximum_request_length(c: *mut xcb_connection_t);
    fn xcb_wait_for_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
    fn xcb_poll_for_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
    fn xcb_request_check(
        c: *mut xcb_connection_t,
        void_cookie: xcb_void_cookie_t
    ) -> *mut xcb_generic_error_t;
    fn xcb_discard_reply64(c: *mut xcb_connection_t, sequence: u64);
    fn xcb_get_setup(c: *mut xcb_connection_t) -> *const xcb_setup_t;
    #[cfg(unix)]
    fn xcb_get_file_descriptor(c: *mut xcb_connection_t) -> c_int;
    fn xcb_connection_has_error(c: *mut xcb_connection_t) -> c_int;
    fn xcb_disconnect(c: *mut xcb_connection_t);
    fn xcb_connect(
        displayname: *const c_char,
        screenp: *mut c_int
    ) -> *mut xcb_connection_t;
    fn xcb_generate_id(c: *mut xcb_connection_t) -> u32;

    // From xcbext.h
    fn xcb_send_request64(
        c: *mut xcb_connection_t,
        flags: c_int,
        vector: *mut iovec,
        request: *const xcb_protocol_request_t
    ) -> u64;
    #[cfg(unix)]
    fn xcb_send_request_with_fds64(
        c: *mut xcb_connection_t,
        flags: c_int,
        vector: *mut iovec,
        request: *const xcb_protocol_request_t,
        num_fds: c_uint,
        fds: *mut c_int
    ) -> u64;
    fn xcb_wait_for_reply64(
        c: *mut xcb_connection_t,
        request: u64,
        e: *mut *mut xcb_generic_error_t
    ) -> *mut c_void;
    fn xcb_poll_for_reply64(
        c: *mut xcb_connection_t,
        request: u64,
        reply: *mut *mut c_void,
        error: *mut *mut xcb_generic_error_t
    ) -> c_int;
}

#[cfg(test)]
mod mock {
    use std::ffi::CStr;

    use libc::{c_char, c_int, c_uint, c_void};

    use super::{
        iovec, xcb_connection_t, xcb_generic_error_t, xcb_generic_event_t, xcb_protocol_request_t,
        xcb_setup_t, xcb_void_cookie_t,
    };
    use crate::protocol::xproto::{ImageOrder, Setup};
    use crate::x11_utils::Serialize;

    #[repr(C)]
    struct ConnectionMock {
        error: c_int,
        setup: Vec<u8>,
    }

    // From xcb.h
    pub(crate) unsafe fn xcb_flush(_c: *mut xcb_connection_t) -> c_int {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_get_maximum_request_length(_c: *mut xcb_connection_t) -> u32 {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_prefetch_maximum_request_length(_c: *mut xcb_connection_t) {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_wait_for_event(_c: *mut xcb_connection_t) -> *mut xcb_generic_event_t {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_poll_for_event(_c: *mut xcb_connection_t) -> *mut xcb_generic_event_t {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_request_check(
        _c: *mut xcb_connection_t,
        _void_cookie: xcb_void_cookie_t,
    ) -> *mut xcb_generic_error_t {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_discard_reply64(_c: *mut xcb_connection_t, _sequence: u64) {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_get_setup(c: *mut xcb_connection_t) -> *const xcb_setup_t {
        // The pointer is suitable aligned since our xcb_connect() mock above created it
        #[allow(clippy::cast_ptr_alignment)]
        ((*(c as *const ConnectionMock)).setup.as_ptr() as _)
    }

    #[cfg(unix)]
    pub(crate) unsafe fn xcb_get_file_descriptor(_c: *mut xcb_connection_t) -> c_int {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_connection_has_error(c: *mut xcb_connection_t) -> c_int {
        // The pointer is suitable aligned since our xcb_connect() mock above created it
        #[allow(clippy::cast_ptr_alignment)]
        (*(c as *const ConnectionMock)).error
    }

    pub(crate) unsafe fn xcb_disconnect(c: *mut xcb_connection_t) {
        // The pointer is suitable aligned since our xcb_connect() mock above created it
        #[allow(clippy::cast_ptr_alignment)]
        let _ = Box::from_raw(c as *mut ConnectionMock);
    }

    pub(crate) unsafe fn xcb_connect(
        displayname: *const c_char,
        screenp: *mut c_int,
    ) -> *mut xcb_connection_t {
        // Test that the provided displayname is correct
        if CStr::from_ptr(displayname).to_str().unwrap() != "display name" {
            panic!("Did not get the expected displayname");
        }
        std::ptr::write(screenp, 0);

        let length_field = 10;
        let setup = Setup {
            status: 0,
            protocol_major_version: 0,
            protocol_minor_version: 0,
            length: length_field,
            release_number: 0,
            resource_id_base: 0,
            resource_id_mask: 0,
            motion_buffer_size: 0,
            maximum_request_length: 0,
            image_byte_order: ImageOrder::LSBFirst,
            bitmap_format_bit_order: ImageOrder::LSBFirst,
            bitmap_format_scanline_unit: 0,
            bitmap_format_scanline_pad: 0,
            min_keycode: 0,
            max_keycode: 0,
            vendor: Default::default(),
            pixmap_formats: Default::default(),
            roots: Default::default(),
        };
        let setup = setup.serialize();
        assert_eq!(setup.len(), 4 * length_field as usize);

        let mock = ConnectionMock { error: 0, setup };
        Box::into_raw(Box::new(mock)) as _
    }

    pub(crate) unsafe fn xcb_generate_id(_c: *mut xcb_connection_t) -> u32 {
        unimplemented!();
    }

    // From xcbext.h
    pub(crate) unsafe fn xcb_send_request64(
        _c: *mut xcb_connection_t,
        _flags: c_int,
        _vector: *mut iovec,
        _request: *const xcb_protocol_request_t,
    ) -> u64 {
        unimplemented!();
    }

    #[cfg(unix)]
    pub(crate) unsafe fn xcb_send_request_with_fds64(
        _c: *mut xcb_connection_t,
        _flags: c_int,
        _vector: *mut iovec,
        _request: *const xcb_protocol_request_t,
        _num_fds: c_uint,
        _fds: *mut c_int,
    ) -> u64 {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_wait_for_reply64(
        _c: *mut xcb_connection_t,
        _request: u64,
        _e: *mut *mut xcb_generic_error_t,
    ) -> *mut c_void {
        unimplemented!();
    }

    pub(crate) unsafe fn xcb_poll_for_reply64(
        _c: *mut xcb_connection_t,
        _request: u64,
        _reply: *mut *mut c_void,
        _error: *mut *mut xcb_generic_error_t,
    ) -> c_int {
        unimplemented!();
    }
}

#[cfg(test)]
pub(crate) use mock::*;
