// SPDX-FileCopyrightText: 2023 Ledger SAS
// SPDX-License-Identifier: Apache-2.0

use core::ptr::addr_of_mut;

const EXCHANGE_AREA_LEN: usize = 128; // TODO: replace by CONFIG-defined value

#[unsafe(link_section = ".svcexchange")]
static mut EXCHANGE_AREA: [u8; EXCHANGE_AREA_LEN] = [0u8; EXCHANGE_AREA_LEN];

/// test purpose, before moving this crate as uapi module. This
/// type is defined in the sentry-kernel uapi types module
pub enum Status {
    Ok,
    Invalid,
}

/// test purpose, before moving this crate as uapi module. This
/// type is defined in the sentry-kernel uapi types module, with extern(C)
/// it order to be readable by the kernel
#[derive(PartialEq,Debug)]
pub struct ShmInfo {
    handle: u32,
    label: u32,
    base: usize,
    len: usize,
    perms: u32,
}

/// Opaque Exchange zone manipulation object
pub struct Area { }

/// Public interface to manipulate the kernel/user exchange zone
///
/// The exchange zone is a specific shared memory zone that is used by
/// the Sentry kernel in order to exchange non-scalar data with the
/// userspace.
///
/// User job can read from or write to it when calling the UAPI syscalls.
/// When a syscall needs some user non-scalar input data, the job needs to
/// write the given data to this zone before calling the syscall.
///
/// In the same way, when retreiving data from the Sentry kernel, the
/// job needs to read back the data from this zone after the syscall has returns.
///
/// The set of potential types that may transit from this zone is build-time
/// known. As a consequence, the trait implementation is made for all
/// potential types, so that the area manipulation is naturally made for all
/// supported kernel/user shared types.
///
pub trait ExhangeArea<T : ?Sized> {

    /// copy vector object to area. length defines the number of T-typed
    /// data that need to be copied to the shared area.
    /// This is typically used when exchanging strings in the println!
    /// upper layer implementation
    fn copy_vec_to(&self, _from: *const T, _length: usize) -> Status {
        Status::Invalid
    }

    /// copy vector object from area. length defines the number of T-typed
    /// data to be copied to the user job T object.
    fn copy_vec_from(&self, _from: *mut T, _length: usize) -> Status {
        Status::Invalid
    }
    /// copy single object of type T to area. This method is used when
    /// delivering user structured data to the kernel.
    fn copy_to(&self, _from: *const T) -> Status {
        Status::Invalid
    }

    /// copy single object of type T from area. This method is used when
    /// receiving structured data from the kernel.
    fn copy_from(&self, _from: *mut T) -> Status {
        Status::Invalid
    }

    /// area length in bytes. Can be used to check that the T-typed data
    /// is small enough to be exchanged with the kernel. This implementation
    /// should be the final one and do not need to be defined in various impl.
    fn area_length(&self) -> usize {
        EXCHANGE_AREA_LEN
    }
}

/// Copy ShmInfo from and to the area.
///
/// In Sentry real world usage, this structure is returned by the kernel, and
/// is never written in the area by the userspace job.
/// The copy_to() is used for test purpose only.
impl ExhangeArea<ShmInfo> for Area {

    #[allow(static_mut_refs)]
    fn copy_from(&self, to: *mut ShmInfo) -> Status {
        unsafe {
            core::ptr::copy_nonoverlapping(
                EXCHANGE_AREA.as_ptr(),
                to as *mut u8,
                core::mem::size_of::<ShmInfo>().min(EXCHANGE_AREA_LEN),
            );
        }
        Status::Ok
    }

    #[allow(static_mut_refs)]
    fn copy_to(&self, from: *const ShmInfo) -> Status {
        unsafe {
            core::ptr::copy_nonoverlapping(
                from as *const u8,
                EXCHANGE_AREA.as_mut_ptr(),
                core::mem::size_of::<ShmInfo>().min(EXCHANGE_AREA_LEN),
            );
        }
        Status::Ok
    }
}

/// Copy u8 vector from and to the area.
///
/// The copy_to() and copy_from() is not implemented as there is no need,
/// by now, for single u8 copy.
impl ExhangeArea<u8> for Area {

    #[allow(static_mut_refs)]
    fn copy_vec_to(&self, from: *const u8, length: usize) -> Status {
        unsafe {
            if Area::check_overlapping(from, length).is_err() {
                return Status::Invalid;
            }
            core::ptr::copy_nonoverlapping(
                from,
                EXCHANGE_AREA.as_mut_ptr(),
                length.min(EXCHANGE_AREA_LEN),
            );
        }
        Status::Ok
    }

    #[allow(static_mut_refs)]
    fn copy_vec_from(&self, to: *mut u8, length: usize) -> Status {
        unsafe {
            if Area::check_overlapping(to, length).is_err() {
                return Status::Invalid;
            }
            core::ptr::copy_nonoverlapping(
                EXCHANGE_AREA.as_ptr(),
                to,
                length.min(EXCHANGE_AREA_LEN),
            );
        }
        Status::Ok
    }
}

/// Non-trait relative utility functions implementation for Area
///
/// Here are defined local functions only, used as helper for trait methods
/// implementations.
impl Area {

    /// create a new Area object. By now, there is no specific metadata in this
    /// object
    fn new() -> Self {
        Self { }
    }

    /// check that the given vector do not overlap with the exchange area
    ///
    /// This is required in order to use the cop_nonoverlapping() API safely.
    #[allow(static_mut_refs)]
    unsafe fn check_overlapping(pointer: *const u8, length: usize) -> Result<(), ()> {
        let area = EXCHANGE_AREA.as_ptr();
        let area_end = area.add(EXCHANGE_AREA_LEN);

        // buffer starts in the middle of the exchange area, abort
        if pointer >= area && pointer <= area_end {
            return Err(());
        }

        // buffer ends in the exchange area, abort
        // Note: this is unlikely to happen if `svc_exchange` is always assumed to be at
        // the beginning of RAM
        let buffer_end = pointer.add(length);
        if buffer_end >= area && buffer_end <= area_end {
            return Err(());
        }

        // exchange area is contained within the buffer, abort
        if pointer <= area && buffer_end >= area_end {
            return Err(());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_area() {
        let area = Area::new();
        assert_eq!(<Area as ExhangeArea<u8>>::area_length(&area), 128);
    }

    #[test]
    fn back_to_back_copy() {
        let area = Area::new();
        let string = [b'z'; 100];
        let mut res = [b'a'; 100];
        area.copy_vec_to(string.as_ptr(), string.len());
        area.copy_vec_from(res.as_mut_ptr(), string.len());
        assert_eq!(res, string);
    }

    #[test]
    fn back_to_back_shm_copy() {
        let area = Area::new();
        let shminfo = ShmInfo {
            handle: 2,
            label: 42,
            base: 0x123456,
            len: 64,
            perms: 0x1,
        };
        let mut shminfo_copy = ShmInfo {
            handle: 0,
            label: 0,
            base: 0,
            len: 0,
            perms: 0,
        };
        area.copy_to(&shminfo);
        area.copy_from(&mut shminfo_copy);
        assert_eq!(shminfo, shminfo_copy);
    }
}
