#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_create_destroy() {
        unsafe {
            let mut stream: RocJpegStreamHandle = std::ptr::null_mut();
            assert_eq!(rocJpegStreamCreate(&mut stream), RocJpegStatus::ROCJPEG_STATUS_SUCCESS);
            assert_eq!(rocJpegStreamDestroy(stream), RocJpegStatus::ROCJPEG_STATUS_SUCCESS);
        }
    }
}
