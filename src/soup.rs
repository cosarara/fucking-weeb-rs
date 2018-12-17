extern crate glib;
extern crate glib_sys;
extern crate gobject_sys;
use std::ffi::CString;
use soup_sys::*;
use std::mem::transmute;
use std::boxed::Box as Box_;
use std::error;

unsafe extern "C"
fn finished_trampoline(session: *mut SoupSession, msg: *mut SoupMessage,
                       f: glib_sys::gpointer)
{
    let f: &&(Fn(*mut SoupSession, *mut SoupMessage) + 'static) = transmute(f);
    f(session, msg)
}

#[derive(Debug, Clone)]
struct BadURL;
impl std::fmt::Display for BadURL {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "URL can't be parsed")
    }
}

impl error::Error for BadURL {
    fn description(&self) -> &str {
        "URL can't be parsed"
    }

    fn cause(&self) -> Option<&error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

#[derive(Debug, Clone)]
struct HttpError;
impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "HTTP error")
    }
}

impl error::Error for HttpError {
    fn description(&self) -> &str {
        "HTTP error"
    }

    fn cause(&self) -> Option<&error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

pub fn get_sync(url: &str) -> Result<Vec<u8>, Box<std::error::Error>>
{
    let url = CString::new(url)?;
    let method = CString::new("GET").unwrap();
    let timeout = CString::new("timeout").unwrap();

    unsafe {
        let parsed = soup_uri_new(url.as_ptr());
        if parsed.is_null() {
            return Err(BadURL.into());
        }
	soup_uri_free(parsed);
    }
    //let null = std::ptr::null::<std::ffi::c_void>(); // i mean, both things work
    let null = 0;
    unsafe {
        let session = soup_session_new_with_options(timeout.as_ptr(), 5, null);
        let msg = soup_message_new(method.as_ptr(), url.as_ptr());
        soup_session_send_message(session, msg);
        let msg = *msg;
        let status = msg.status_code;
        debug!("message code {}", status);
        if status >= 200 && status < 300 {
            let body = *msg.response_body;
            let data = std::slice::from_raw_parts(body.data as *const u8, body.length as usize);
            return Ok(data.to_vec())
        } else {
            return Err(HttpError.into());
        }
    }
}

pub fn get_async<F: Fn(Option<&[u8]>) + 'static>(url: &str, f: F) -> Result<(), Box<std::error::Error>>
{
    let url = CString::new(url)?;
    let method = CString::new("GET").unwrap();
    let timeout = CString::new("timeout").unwrap();

    let finished = move |_sess: *mut SoupSession, msg_ptr: *mut SoupMessage| {
        if msg_ptr.is_null() {
            panic!("soup ur dumb");
        }
        let msg = unsafe { *msg_ptr };
        let status = msg.status_code;
        debug!("message code {}", status);
        if status >= 200 && status < 300 {
            let data = unsafe {
                let body = *msg.response_body;
                std::slice::from_raw_parts(body.data as *const u8, body.length as usize)
            };
            f(Some(data));
        } else {
            f(None);
        }
        unsafe {
            gobject_sys::g_object_unref(msg_ptr as *mut gobject_sys::GObject);
        }
    };
    let finished: Box_<Box_<Fn(*mut SoupSession, *mut SoupMessage) + 'static>> =
        Box_::new(Box_::new(finished));

    unsafe {
        let parsed = soup_uri_new(url.as_ptr());
        if parsed.is_null() {
            return Err(BadURL.into());
        }
	soup_uri_free(parsed);
        //let null = std::ptr::null::<std::ffi::c_void>(); // i mean, both things work
        let null = 0;
        let session = soup_session_new_with_options(timeout.as_ptr(), 5, null);
        let msg = soup_message_new(method.as_ptr(), url.as_ptr());
        // TODO: ask some gtk-rs dev about this ref/unref business
        gobject_sys::g_object_ref(msg as *mut gobject_sys::GObject);
        soup_session_queue_message(session, msg, transmute(finished_trampoline as usize),
            Box_::into_raw(finished) as *mut _);
    }

    Ok(())
}

