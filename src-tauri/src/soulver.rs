use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn initialize(soulver_core_path: &str) {
    INIT.call_once(|| {
        let resources_path_str = format!("{}/SoulverCore_SoulverCore.resources", soulver_core_path);
        let resources_path_cstr = CString::new(resources_path_str).expect("CString::new failed");

        unsafe {
            initialize_soulver(resources_path_cstr.as_ptr());
        }
    });
}

#[cfg(not(test))]
#[link(name = "SoulverWrapper", kind = "dylib")]
extern "C" {
    fn initialize_soulver(resourcesPath: *const c_char);
    fn evaluate(expression: *const c_char) -> *mut c_char;
    fn free_string(ptr: *mut c_char);
}

#[cfg(test)]
extern "C" {
    fn initialize_soulver(resourcesPath: *const c_char);
    fn evaluate(expression: *const c_char) -> *mut c_char;
    fn free_string(ptr: *mut c_char);
}

struct StringPtrGuard(*mut c_char);

impl Drop for StringPtrGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { free_string(self.0) };
        }
    }
}

#[tauri::command]
pub fn calculate_soulver(expression: String) -> Result<String, String> {
    let c_expression = CString::new(expression).map_err(|e| e.to_string())?;

    let result_ptr = unsafe { evaluate(c_expression.as_ptr()) };
    let _guard = StringPtrGuard(result_ptr);

    if result_ptr.is_null() {
        return Err("Evaluation failed, received null pointer from Swift.".to_string());
    }

    let result_string = unsafe {
        let c_result = CStr::from_ptr(result_ptr);
        c_result.to_str().map_err(|e| e.to_string())?.to_owned()
    };

    Ok(result_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static FREE_CALLED: AtomicBool = AtomicBool::new(false);
    static mut MOCK_EVAL_RESPONSE: *mut c_char = std::ptr::null_mut();

    #[no_mangle]
    pub extern "C" fn initialize_soulver(_: *const c_char) {}

    #[no_mangle]
    pub extern "C" fn evaluate(_: *const c_char) -> *mut c_char {
        unsafe { MOCK_EVAL_RESPONSE }
    }

    #[no_mangle]
    pub extern "C" fn free_string(ptr: *mut c_char) {
        if !ptr.is_null() {
            FREE_CALLED.store(true, Ordering::SeqCst);
            unsafe {
                let _ = CString::from_raw(ptr);
            }
        }
    }

    fn set_mock_response(response: Option<&str>) {
        FREE_CALLED.store(false, Ordering::SeqCst);
        unsafe {
            if let Some(s) = response {
                MOCK_EVAL_RESPONSE = CString::new(s).unwrap().into_raw();
            } else {
                MOCK_EVAL_RESPONSE = std::ptr::null_mut();
            }
        }
    }

    fn set_invalid_utf8_mock_response() {
        FREE_CALLED.store(false, Ordering::SeqCst);
        unsafe {
            let invalid_utf8: &[u8] = &[0xC3, 0x28, 0x00];
            MOCK_EVAL_RESPONSE = CString::from_vec_unchecked(invalid_utf8.to_vec()).into_raw();
        }
    }

    #[test]
    fn test_calculate_soulver_success() {
        let mock_json = r#"{"value":"15", "type":"Number", "error":null}"#;
        set_mock_response(Some(mock_json));

        let result = calculate_soulver("10 + 5".to_string());

        assert_eq!(result.unwrap(), mock_json);
        assert!(FREE_CALLED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_calculate_soulver_null_pointer_from_ffi() {
        set_mock_response(None);

        let result = calculate_soulver("some expression".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("null pointer"));
        assert!(!FREE_CALLED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_calculate_soulver_invalid_utf8_from_ffi() {
        set_invalid_utf8_mock_response();

        let result = calculate_soulver("some expression".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid utf-8"));
        assert!(FREE_CALLED.load(Ordering::SeqCst));
    }
}
