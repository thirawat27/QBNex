//! Pixel and byte buffers handed to C (screen capture, clipboard).
//!
//! Buffers are boxed slices, so C only needs the pointer and the element
//! count to give them back to `qb_free_pixels` / `qb_free_bytes`.

/// Leaks `v` to C. Returns NULL for an empty vector.
pub fn into_raw(v: Vec<u32>) -> *mut u32 {
    if v.is_empty() {
        return std::ptr::null_mut();
    }
    Box::into_raw(v.into_boxed_slice()).cast()
}

/// Leaks `v` to C. Returns NULL for an empty vector.
pub fn bytes_into_raw(v: Vec<u8>) -> *mut u8 {
    if v.is_empty() {
        return std::ptr::null_mut();
    }
    Box::into_raw(v.into_boxed_slice()).cast()
}

/// Frees a buffer from `into_raw` holding `count` pixels.
///
/// # Safety
/// `p` must come from this module with the same `count`, or be NULL.
#[no_mangle]
pub unsafe extern "C" fn qb_free_pixels(p: *mut u32, count: i64) {
    if !p.is_null() {
        // SAFETY: rebuilds the boxed slice leaked by `into_raw`.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, count as usize)) });
    }
}

/// Frees a buffer from `bytes_into_raw` holding `count` bytes.
///
/// # Safety
/// `p` must come from this module with the same `count`, or be NULL.
#[no_mangle]
pub unsafe extern "C" fn qb_free_bytes(p: *mut u8, count: i64) {
    if !p.is_null() {
        // SAFETY: rebuilds the boxed slice leaked by `bytes_into_raw`.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, count as usize)) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = into_raw(vec![1, 2, 3]);
        assert_eq!(unsafe { *p.add(2) }, 3);
        unsafe { qb_free_pixels(p, 3) };
        let b = bytes_into_raw(b"abc".to_vec());
        unsafe { qb_free_bytes(b, 3) };
        assert!(into_raw(Vec::new()).is_null());
        unsafe { qb_free_pixels(std::ptr::null_mut(), 0) };
    }
}
