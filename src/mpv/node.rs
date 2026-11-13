//! Reading mpv's structured property values (`mpv_node`): one read for a whole list,
//! instead of a synchronous read per field.
//!
//! Every function here takes a node that must be valid: filled by `mpv_get_property` and
//! not yet freed, or built with valid pointers of the format it declares.

use std::ffi::CStr;

use libmpv_sys::*;

use super::Handle;

/// Runs `f` on property `name` read as a node; `None` while mpv can't provide it.
pub(super) fn read<T>(mpv: Handle, name: &CStr, f: impl FnOnce(&mpv_node) -> T) -> Option<T> {
    let mut node: mpv_node = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        mpv_get_property(
            mpv.0,
            name.as_ptr(),
            mpv_format_MPV_FORMAT_NODE,
            (&raw mut node).cast(),
        )
    };
    if rc < 0 {
        return None;
    }
    let value = f(&node);
    unsafe { mpv_free_node_contents(&raw mut node) };
    Some(value)
}

/// Items of an array or map node; empty for any other node.
///
/// # Safety
/// `node` must be a valid mpv node.
pub(super) unsafe fn list(node: &mpv_node) -> &[mpv_node] {
    let is_list = node.format == mpv_format_MPV_FORMAT_NODE_ARRAY
        || node.format == mpv_format_MPV_FORMAT_NODE_MAP;
    let list = if is_list {
        unsafe { node.u.list.as_ref() }
    } else {
        None
    };
    match list {
        Some(l) if l.num > 0 && !l.values.is_null() => unsafe {
            std::slice::from_raw_parts(l.values, l.num as usize)
        },
        _ => &[],
    }
}

/// Value under `key` in a map node.
///
/// # Safety
/// `node` must be a valid mpv node.
pub(super) unsafe fn get<'a>(node: &'a mpv_node, key: &str) -> Option<&'a mpv_node> {
    if node.format != mpv_format_MPV_FORMAT_NODE_MAP {
        return None;
    }
    let list = unsafe { node.u.list.as_ref() }?;
    let values = unsafe { self::list(node) };
    values.iter().enumerate().find_map(|(i, value)| {
        let k = unsafe { CStr::from_ptr(*list.keys.add(i)) };
        (k.to_bytes() == key.as_bytes()).then_some(value)
    })
}

/// String under `key` in a map node.
///
/// # Safety
/// `node` must be a valid mpv node.
pub(super) unsafe fn string(node: &mpv_node, key: &str) -> Option<String> {
    let value = unsafe { get(node, key) }?;
    (value.format == mpv_format_MPV_FORMAT_STRING).then(|| {
        unsafe { CStr::from_ptr(value.u.string) }
            .to_string_lossy()
            .into_owned()
    })
}

/// Integer under `key` in a map node.
///
/// # Safety
/// `node` must be a valid mpv node.
pub(super) unsafe fn int(node: &mpv_node, key: &str) -> Option<i64> {
    let value = unsafe { get(node, key) }?;
    if value.format == mpv_format_MPV_FORMAT_INT64 {
        Some(unsafe { value.u.int64 })
    } else {
        None
    }
}

/// Flag under `key` in a map node.
///
/// # Safety
/// `node` must be a valid mpv node.
pub(super) unsafe fn flag(node: &mpv_node, key: &str) -> Option<bool> {
    let value = unsafe { get(node, key) }?;
    if value.format == mpv_format_MPV_FORMAT_FLAG {
        Some(unsafe { value.u.flag } != 0)
    } else {
        None
    }
}

/// Builds nodes the way mpv fills them, for tests; leaks, which is fine there.
#[cfg(test)]
pub(super) mod build {
    use std::ffi::{CString, c_char};

    use libmpv_sys::*;

    use mpv_node__bindgen_ty_1 as Value;

    fn node(format: mpv_format, u: Value) -> mpv_node {
        mpv_node { u, format }
    }

    pub fn string(value: &str) -> mpv_node {
        let string = CString::new(value).unwrap().into_raw();
        node(mpv_format_MPV_FORMAT_STRING, Value { string })
    }

    pub fn int(int64: i64) -> mpv_node {
        node(mpv_format_MPV_FORMAT_INT64, Value { int64 })
    }

    pub fn flag(value: bool) -> mpv_node {
        let flag = value.into();
        node(mpv_format_MPV_FORMAT_FLAG, Value { flag })
    }

    pub fn array(values: Vec<mpv_node>) -> mpv_node {
        let keys = std::ptr::null_mut();
        list(mpv_format_MPV_FORMAT_NODE_ARRAY, values, keys)
    }

    pub fn map(entries: Vec<(&str, mpv_node)>) -> mpv_node {
        let (keys, values): (Vec<*mut c_char>, Vec<mpv_node>) = entries
            .into_iter()
            .map(|(key, value)| (CString::new(key).unwrap().into_raw(), value))
            .unzip();
        let keys = keys.leak().as_mut_ptr();
        list(mpv_format_MPV_FORMAT_NODE_MAP, values, keys)
    }

    fn list(format: mpv_format, values: Vec<mpv_node>, keys: *mut *mut c_char) -> mpv_node {
        let list = Box::leak(Box::new(mpv_node_list {
            num: values.len() as i32,
            values: values.leak().as_mut_ptr(),
            keys,
        }));
        node(format, Value { list })
    }
}
