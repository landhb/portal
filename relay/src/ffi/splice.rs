use crate::errors::RelayError;
use std::convert::TryInto;
use std::os::unix::io::RawFd;

pub fn splice(infd: RawFd, outfd: RawFd) -> Result<usize, RelayError> {
    // Make the syscall
    let res = unsafe {
        libc::splice(
            infd,
            std::ptr::null_mut::<libc::loff_t>(),
            outfd,
            std::ptr::null_mut::<libc::loff_t>(),
            crate::MAX_SPLICE_SIZE,
            libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK,
        )
    };

    // Return the underlying error
    if res < 0 {
        return Err(RelayError::from_errno());
    }

    // Return the amount spliced
    Ok(res.try_into().or(Err(RelayError::TypeError))?)
}
